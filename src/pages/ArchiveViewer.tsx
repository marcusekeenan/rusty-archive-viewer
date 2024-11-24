import {
  createSignal,
  createEffect,
  createMemo,
  onMount,
  onCleanup,
  Show,
} from "solid-js";
import { createStore } from "solid-js/store";
import { ErrorBoundary } from "solid-js";

import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import AxisManager from "../components/controls/AxisManager";
import ControlPanel from "../components/controls/ControlPanel";
import ChartJS from "../components/chart/ChartJS";
import ConnectionStatus from "../components/controls/ConnectionStatus";

import {
  fetchData,
  LiveUpdateManager,
  getPVMetadata,
  testConnection,
} from "../utils/archiverApi";

import type {
  PVWithProperties,
  PenProperties,
} from "../components/controls/types";
import type { AxisConfig } from "../components/chart/types";
import type { NormalizedPVData, PointValue, Meta } from "../types/api";
import { DataOperator } from "../types/api";

// Constants
const CONNECTION_CHECK_INTERVAL = 30000; // 30 seconds

const DEFAULT_UPDATE_INTERVAL = 1000; // 1 second in milliseconds
const MIN_UPDATE_INTERVAL = 1000; // Minimum 1 second
const MAX_UPDATE_INTERVAL = 30000; // Maximum 30 seconds

interface ViewerState {
  selectedPVs: PVWithProperties[];
  visiblePVs: Set<string>;
  timeRange: {
    start: Date;
    end: Date;
  };
  data: NormalizedPVData[];
  loading: boolean;
  error: string | null;
  lastRefresh: Date | null;
  timezone: string;
  liveModeConfig: {
    enabled: boolean;
    mode: "rolling" | "append";
    updateInterval: number;
  };
  axes: Map<string, AxisConfig>;
  dataOperator: DataOperator;
  isConnected: boolean;
}

const INITIAL_STATE: ViewerState = {
  selectedPVs: [],
  visiblePVs: new Set(),
  timeRange: {
    start: new Date(Date.now() - 3600000),
    end: new Date(),
  },
  data: [],
  loading: false,
  error: null,
  lastRefresh: null,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  liveModeConfig: {
    enabled: false,
    mode: "rolling",
    updateInterval: DEFAULT_UPDATE_INTERVAL,
  },
  axes: new Map(),
  dataOperator: DataOperator.Optimized,
  isConnected: true,
};

export default function ArchiveViewer() {
  let liveManager: LiveUpdateManager | undefined;
  let connectionCheckInterval: number;

  const [state, setState] = createStore<ViewerState>(INITIAL_STATE);

  const visibleData = createMemo(() =>
    state.data.filter((pv) => state.visiblePVs.has(pv.meta.name))
  );

  const checkConnection = async () => {
    try {
      await testConnection();
      setState("isConnected", true);
    } catch (error) {
      setState("isConnected", false);
      console.error("Connection check failed:", error);
    }
  };

  const updatePVMetadata = (pvName: string, metadata: Meta) => {
    const displayLimits =
      metadata.display_limits ||
      (metadata.display_high !== undefined && metadata.display_low !== undefined
        ? { low: metadata.display_low, high: metadata.display_high }
        : { low: -100, high: 100 });

    const egu = metadata.egu || "Value";
    const existingAxis = Array.from(state.axes.values()).find(
      (axis) => axis.egu.toLowerCase() === egu.toLowerCase()
    );

    let axisId: string;

    if (existingAxis) {
      axisId = existingAxis.id;
    } else {
      axisId = `axis_${egu.toLowerCase().replace(/[^a-z0-9]/g, "_")}_${Date.now()}`;
      setState("axes", (axes) => {
        const newAxes = new Map(axes);
        newAxes.set(axisId, {
          id: axisId,
          egu,
          position: newAxes.size % 2 === 0 ? "left" : "right",
          autoRange: true,
          range: displayLimits,
          pvs: new Set([pvName]),
        });
        return newAxes;
      });
    }

    setState("selectedPVs", (pvs) =>
      pvs.map((pv) =>
        pv.name === pvName
          ? {
              ...pv,
              metadata: {
                ...metadata,
                egu,
                display_limits: displayLimits,
              },
              axisId,
            }
          : pv
      )
    );

    if (existingAxis) {
      setState("axes", (axes) => {
        const newAxes = new Map(axes);
        const axis = newAxes.get(axisId);
        if (axis) {
          axis.pvs.add(pvName);
        }
        return newAxes;
      });
    }
  };

  const processLiveData = (pointValues: Record<string, PointValue>) => {
    setState((s) => {
      const newData = s.data.map((pvData) => {
        const newPoint = pointValues[pvData.meta.name];
        if (!newPoint) return pvData;
  
        const value = typeof newPoint.val === "number"
          ? newPoint.val
          : Array.isArray(newPoint.val)
            ? newPoint.val[0]
            : null;
        if (value === null) return pvData;
  
        const timestamp = newPoint.secs * 1000 + (newPoint.nanos ? newPoint.nanos / 1_000_000 : 0);
        const lastPoint = pvData.data[pvData.data.length - 1];
  
        // Don't add points if:
        // 1. We already have this exact timestamp
        // 2. We have a point with the same value and it's within the update interval
        if (lastPoint) {
          const timeDiff = timestamp - lastPoint.timestamp;
          const sameValue = Math.abs(lastPoint.value - value) < 1e-10; // Use small epsilon for float comparison
          
          if (timestamp === lastPoint.timestamp || 
              (sameValue && timeDiff < s.liveModeConfig.updateInterval)) {
            return pvData;
          }
        }
  
        let newPvData = { ...pvData };
        const cutoffTime = s.liveModeConfig.mode === "rolling"
          ? Date.now() - (s.timeRange.end.getTime() - s.timeRange.start.getTime())
          : 0;
  
        // If there's a significant gap between points, add an interpolation point
        if (lastPoint && (timestamp - lastPoint.timestamp) > s.liveModeConfig.updateInterval * 2) {
          // Only add interpolation if the value changed
          if (Math.abs(lastPoint.value - value) > 1e-10) {
            const interpolationPoint = {
              timestamp: lastPoint.timestamp + s.liveModeConfig.updateInterval,
              severity: lastPoint.severity,
              status: lastPoint.status,
              value: lastPoint.value,
              min: lastPoint.value,
              max: lastPoint.value,
              stddev: 0,
              count: 1,
            };
            newPvData.data = [...pvData.data, interpolationPoint];
          }
        }
  
        // Add the new point
        const newPointData = {
          timestamp,
          severity: newPoint.severity || 0,
          status: newPoint.status || 0,
          value,
          min: value,
          max: value,
          stddev: 0,
          count: 1,
        };
  
        newPvData.data = [...(newPvData.data || pvData.data), newPointData];
  
        // Clean up old points in rolling mode
        if (s.liveModeConfig.mode === "rolling") {
          newPvData.data = newPvData.data.filter(point => point.timestamp >= cutoffTime);
        }
  
        // Sort points to ensure correct order
        newPvData.data.sort((a, b) => a.timestamp - b.timestamp);
  
        return newPvData;
      });
  
      const now = new Date();
      return {
        data: newData,
        timeRange: s.liveModeConfig.mode === "rolling"
          ? {
              start: new Date(now.getTime() - (s.timeRange.end.getTime() - s.timeRange.start.getTime())),
              end: now,
            }
          : { ...s.timeRange, end: now },
        lastRefresh: now,
      };
    });
  };

  const fetchDataForPVs = async () => {
    if (state.selectedPVs.length === 0) return;

    const timeRangeSeconds = Math.floor(
      (state.timeRange.end.getTime() - state.timeRange.start.getTime()) / 1000
    );

    const now = new Date();
    const end =
      state.liveModeConfig.mode === "rolling" ? now : state.timeRange.end;
    const start =
      state.liveModeConfig.mode === "rolling"
        ? new Date(now.getTime() - timeRangeSeconds * 1000)
        : state.timeRange.start;

    setState("loading", true);
    try {
      const data = await fetchData(
        state.selectedPVs.map((pv) => pv.name),
        start,
        end,
        {
          timezone: state.timezone,
          mode: state.liveModeConfig.mode,
          operator: state.dataOperator,
          fetchLatestMetadata: true,
        }
      );

      const filteredData = data.map((pvData) => ({
        ...pvData,
        data: pvData.data.filter(
          (point) =>
            point.timestamp >= start.getTime() &&
            point.timestamp <= end.getTime()
        ),
      }));

      setState({
        data: filteredData,
        error: null,
        isConnected: true,
        lastRefresh: new Date(),
        timeRange: { start, end },
      });

      filteredData.forEach((pvData) => {
        if (pvData.meta) {
          updatePVMetadata(pvData.meta.name, pvData.meta);
        }
      });
    } catch (error) {
      console.error("Fetch error:", error);
      setState({
        error: String(error),
        isConnected: false,
      });
    } finally {
      setState("loading", false);
    }
  };

  const calculateOptimalInterval = (data: NormalizedPVData[]): number => {
    if (!data.length) return DEFAULT_UPDATE_INTERVAL;

    const intervals = data.map((pvData) => {
      if (pvData.data.length < 2) return DEFAULT_UPDATE_INTERVAL;

      // Calculate time differences between consecutive points
      const timestamps = pvData.data.map((p) => p.timestamp);
      const differences = timestamps
        .slice(1)
        .map((t, i) => t - timestamps[i])
        .filter((diff) => diff > 0); // Filter out zero differences

      if (!differences.length) return DEFAULT_UPDATE_INTERVAL;

      // Use the median difference to avoid outliers
      differences.sort((a, b) => a - b);
      const medianIndex = Math.floor(differences.length / 2);
      return differences[medianIndex];
    });

    // Get the maximum interval among all PVs, but within reasonable bounds
    const optimalInterval = Math.min(
      Math.max(Math.max(...intervals), MIN_UPDATE_INTERVAL),
      MAX_UPDATE_INTERVAL
    );

    // Round to the nearest second
    return Math.round(optimalInterval / 1000) * 1000;
  };
  const toggleLiveMode = async () => {
    if (state.liveModeConfig.enabled) {
      console.log("Stopping live mode");
      await liveManager?.stop();
      setState("liveModeConfig", "enabled", false);
    } else {
      console.log("Starting live mode");

      if (!state.isConnected) {
        await checkConnection();
        if (!state.isConnected) {
          setState("error", "Cannot start live mode: No connection to server");
          return;
        }
      }

      const now = new Date();
      const lastDataTime = state.data.reduce((latest, pv) => {
        const pvLastPoint = pv.data[pv.data.length - 1];
        return pvLastPoint ? Math.max(latest, pvLastPoint.timestamp) : latest;
      }, state.timeRange.end.getTime());

      let updateInterval = DEFAULT_UPDATE_INTERVAL;

      if (now.getTime() - lastDataTime > 0) {
        try {
          const gapData = await fetchData(
            state.selectedPVs.map((pv) => pv.name),
            new Date(lastDataTime),
            now,
            {
              timezone: state.timezone,
              mode: "fixed",
              operator: state.dataOperator,
              fetchLatestMetadata: true,
            }
          );

          setState("data", (data) =>
            data.map((pvData) => {
              const gapPvData = gapData.find(
                (g) => g.meta.name === pvData.meta.name
              );
              if (!gapPvData) return pvData;

              return {
                ...pvData,
                data: [
                  ...pvData.data,
                  ...gapPvData.data.filter(
                    (point) =>
                      point.timestamp >
                      pvData.data[pvData.data.length - 1].timestamp
                  ),
                ],
              };
            })
          );

          // Calculate optimal update interval from gap data
          updateInterval = calculateOptimalInterval(gapData);
        } catch (error) {
          console.warn("Failed to fetch gap data:", error);
          // Use default interval if gap fetch fails
          updateInterval = DEFAULT_UPDATE_INTERVAL;
        }
      }

      // Ensure we have a valid update interval
      updateInterval = Math.max(
        MIN_UPDATE_INTERVAL,
        Math.min(updateInterval, MAX_UPDATE_INTERVAL)
      );
      console.log(`Using update interval: ${updateInterval}ms`);

      // Update the state with the new interval
      setState("liveModeConfig", "updateInterval", updateInterval);

      // Create and start the live manager
      liveManager = new LiveUpdateManager();
      await liveManager.start({
        pvs: state.selectedPVs.map((pv) => pv.name),
        updateIntervalMs: updateInterval,
        timezone: state.timezone,
        onData: processLiveData,
      });

      setState("liveModeConfig", "enabled", true);

      if (state.liveModeConfig.mode === "rolling") {
        const duration =
          state.timeRange.end.getTime() - state.timeRange.start.getTime();
        setState("timeRange", {
          start: new Date(now.getTime() - duration),
          end: now,
        });
      }
    }
  };

  const handleAxisAssignment = (pvName: string, newAxisId: string) => {
    setState("axes", (axes) => {
      const newAxes = new Map(axes);
      for (const axis of newAxes.values()) {
        axis.pvs.delete(pvName);
      }
      const targetAxis = newAxes.get(newAxisId);
      if (targetAxis) {
        targetAxis.pvs.add(pvName);
      }
      return newAxes;
    });
  };

  onMount(() => {
    checkConnection();
    connectionCheckInterval = window.setInterval(
      checkConnection,
      CONNECTION_CHECK_INTERVAL
    );
  });

  onCleanup(() => {
    if (connectionCheckInterval) {
      window.clearInterval(connectionCheckInterval);
    }
    liveManager?.stop();
  });

  createEffect(() => {
    if (!state.isConnected && state.liveModeConfig.enabled) {
      liveManager?.stop();
      setState("liveModeConfig", "enabled", false);
    }
  });

  return (
    <ErrorBoundary fallback={(err) => <div>Error: {err.toString()}</div>}>
      <div class="grid grid-cols-[300px_1fr_300px] gap-4 p-4 bg-gray-50 min-h-screen">
        <div class="space-y-4">
          <PVSelector
            selectedPVs={() => state.selectedPVs}
            visiblePVs={() => state.visiblePVs}
            availableAxes={state.axes}
            onAddPV={async (pv, properties) => {
              setState("selectedPVs", (pvs) => [
                ...pvs,
                {
                  name: pv,
                  pen: properties,
                },
              ]);
              setState("visiblePVs", (pvs) => new Set([...pvs, pv]));

              try {
                const metadata = await getPVMetadata(pv);
                if (metadata) {
                  updatePVMetadata(pv, metadata);
                }
              } catch (error) {
                console.warn(`Failed to fetch metadata for ${pv}`, error);
                const defaultMetadata: Meta = {
                  name: pv,
                  egu: "Value",
                  description: "No metadata available",
                  display_limits: { low: -100, high: 100 },
                };
                updatePVMetadata(pv, defaultMetadata);
              }

              fetchDataForPVs();
            }}
            onUpdatePV={(pv, properties, axisId) => {
              setState("selectedPVs", (pvs) =>
                pvs.map((p) =>
                  p.name === pv ? { ...p, pen: properties, axisId } : p
                )
              );
              handleAxisAssignment(pv, axisId);
            }}
            onRemovePV={(pv) => {
              setState((s) => {
                const newPvs = s.selectedPVs.filter((p) => p.name !== pv);
                const newVisible = new Set(s.visiblePVs);
                newVisible.delete(pv);
                const axes = new Map(s.axes);
                for (const [axisId, axis] of axes.entries()) {
                  axis.pvs.delete(pv);
                  if (axis.pvs.size === 0) {
                    axes.delete(axisId);
                  }
                }
                return {
                  selectedPVs: newPvs,
                  visiblePVs: newVisible,
                  axes,
                };
              });
            }}
            onVisibilityChange={(pv, isVisible) => {
              setState("visiblePVs", (pvs) => {
                const newPvs = new Set(pvs);
                isVisible ? newPvs.add(pv) : newPvs.delete(pv);
                return newPvs;
              });
            }}
          />

          <AxisManager
            axes={() => state.axes}
            onAxisEdit={(updatedAxis) => {
              setState("axes", (axes) => {
                const newAxes = new Map(axes);
                newAxes.set(updatedAxis.id, updatedAxis);
                return newAxes;
              });
            }}
            onAxisAdd={(newAxis) => {
              setState("axes", (axes) => {
                const newAxes = new Map(axes);
                newAxes.set(newAxis.id, newAxis);
                return newAxes;
              });
            }}
            onAxisRemove={(axisId) => {
              setState("axes", (axes) => {
                const axis = axes.get(axisId);
                if (!axis || axis.pvs.size > 0) return axes;
                const newAxes = new Map(axes);
                newAxes.delete(axisId);
                return newAxes;
              });
            }}
          />
        </div>

        <div class="space-y-4">
          <ControlPanel
            liveModeConfig={() => state.liveModeConfig}
            dataOperator={() => state.dataOperator}
            loading={() => state.loading}
            onLiveModeToggle={toggleLiveMode}
            onLiveModeConfigChange={(config) => {
              setState("liveModeConfig", config);
              fetchDataForPVs();
            }}
            onDataOperatorChange={(operator) => {
              setState("dataOperator", operator);
              fetchDataForPVs();
            }}
            onRefresh={fetchDataForPVs}
            onExport={() => {}} // Removed export functionality
          />

          <div class="bg-white rounded-lg shadow-sm p-4">
            <div class="w-full h-[calc(100vh-280px)] relative">
              <Show
                when={visibleData().length > 0}
                fallback={
                  <div class="absolute inset-0 flex items-center justify-center text-gray-400">
                    No data to display
                  </div>
                }
              >
                <ChartJS
                  data={visibleData()}
                  pvs={state.selectedPVs.filter((pv) =>
                    state.visiblePVs.has(pv.name)
                  )}
                  timeRange={state.timeRange}
                  timezone={state.timezone}
                  axes={state.axes}
                />
              </Show>
            </div>
          </div>
        </div>

        <div class="space-y-4">
          <TimeRangeSelector
            onChange={(start, end, timezone) => {
              setState({
                timeRange: { start, end },
                timezone,
              });
              fetchDataForPVs();
            }}
            disabled={state.loading}
            initialTimezone={state.timezone}
            currentStartDate={state.timeRange.start}
            currentEndDate={state.timeRange.end}
            isLiveMode={state.liveModeConfig.enabled}
            liveMode={state.liveModeConfig.mode}
          />
        </div>

        <ConnectionStatus isConnected={state.isConnected} />
      </div>
    </ErrorBoundary>
  );
}
