import { createSignal, createEffect, createMemo, onMount, onCleanup, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { createStore } from "solid-js/store";
import { ErrorBoundary } from "solid-js";

import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector"; 
import AxisManager from "../components/controls/AxisManager";
import ControlPanel from "../components/controls/ControlPanel";
import ChartJS from "../components/chart/ChartJS";
import DebugDialog from "./DebugDialog";
import ConfigurationManager from "../components/controls/ConfigurationManager";
import ConnectionStatus from "../components/controls/ConnectionStatus";

import { fetchData, LiveUpdateManager, getPVMetadata } from "../utils/archiverApi";
import { TimeRange, LiveModeConfig } from "../types/common";
import { NormalizedPVData, PointValue, Meta, DataOperator } from "../types/api";
import type { PVWithProperties, PenProperties } from "../components/controls/types";
import type { AxisConfig, AxisAssignment } from "../components/chart/types";
import type { DebugLog } from "../types/debug";

const clearStoredConfigs = () => {
  const keys = [
    "selectedPVs", "visiblePVs", "timeRange", "timezone",
    "liveModeConfig", "axes", "dataOperator", "fetchOptions"
  ];
  keys.forEach(key => localStorage.removeItem(key));
};

const saveState = (key: string, value: any) => {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (error) {
    console.error(`Failed to save state for ${key}:`, error);
  }
};

const loadState = (key: string) => {
  try {
    const stored = localStorage.getItem(key);
    return stored ? JSON.parse(stored) : null;
  } catch (error) {
    console.error(`Failed to load state for ${key}:`, error);
    return null;
  }
};

interface ViewerState {
  selectedPVs: PVWithProperties[];
  visiblePVs: Set<string>;
  timeRange: TimeRange;
  data: NormalizedPVData[];
  loading: boolean;
  error: string | null;
  debugLogs: DebugLog[];
  showDebugData: boolean;
  lastRefresh: Date | null;
  timezone: string;
  liveModeConfig: LiveModeConfig;
  axes: Map<string, AxisConfig>;
  dataOperator: DataOperator;
  fetchOptions: {
    fetchLatestMetadata: boolean;
    useRawProcessing: boolean;
  };
  isConnected: boolean;
  showExportDialog: boolean;
}

const DEFAULT_UPDATE_INTERVAL = 1000;

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
  debugLogs: [],
  showDebugData: false,
  lastRefresh: null,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  liveModeConfig: {
    enabled: false,
    mode: "rolling",
    updateInterval: DEFAULT_UPDATE_INTERVAL,
  },
  axes: new Map(),
  dataOperator: DataOperator.Optimized,
  fetchOptions: {
    fetchLatestMetadata: true,
    useRawProcessing: false,
  },
  isConnected: true,
  showExportDialog: false,
};

export default function ArchiveViewer() {
  const navigate = useNavigate();
  let chartContainer: HTMLDivElement | undefined;
  let liveManager: LiveUpdateManager | undefined;

  const [state, setState] = createStore<ViewerState>(INITIAL_STATE);

  const visibleData = createMemo(() => 
    state.data.filter(pv => state.visiblePVs.has(pv.meta.name))
  );

  const generateAxisId = (egu: string): string => {
    const base = egu.toLowerCase().replace(/[^a-z0-9]/g, "_");
    const existing = Array.from(state.axes.keys()).filter(k => k.startsWith(base));
    return existing.length ? `${base}_${existing.length + 1}` : base;
  };

  const findOrCreateAxis = (metadata: Meta): string => {
    const existingAxis = Array.from(state.axes.values())
      .find(axis => axis.egu.toLowerCase() === metadata.egu.toLowerCase());

    if (existingAxis) {
      return existingAxis.id;
    }

    const axisId = generateAxisId(metadata.egu);
    setState("axes", axes => {
      const newAxes = new Map(axes);
      newAxes.set(axisId, {
        id: axisId,
        egu: metadata.egu,
        position: newAxes.size % 2 === 0 ? 'left' : 'right',
        autoRange: true,
        range: { low: 0, high: 100 }, // Default range
        pvs: new Set([metadata.name]) // Add the PV to the new axis
      });
      return newAxes;
    });

    return axisId;
  };

  const updatePVMetadata = (pvName: string, metadata: Meta) => {
    const axisId = findOrCreateAxis(metadata);
    
    setState("selectedPVs", pvs => {
      const updatedPvs = pvs.map(pv => 
        pv.name === pvName ? { ...pv, metadata, axisId } : pv
      );
      return updatedPvs;
    });

    setState("axes", axes => {
      const newAxes = new Map(axes);
      const axis = newAxes.get(axisId);
      if (axis) {
        axis.pvs.add(pvName);
      }
      return newAxes;
    });
  };

  const processLiveData = (pointValues: Record<string, PointValue>) => {
    console.log("Received live update:", pointValues);
    setState(s => {
      const newData = s.data.map(pvData => {
        const newPoint = pointValues[pvData.meta.name];
        if (!newPoint) return pvData;

        const value = typeof newPoint.val === "number" ? newPoint.val :
                     Array.isArray(newPoint.val) ? newPoint.val[0] : null;
        if (value === null) return pvData;

        const timestamp = newPoint.secs * 1000 + (newPoint.nanos ? newPoint.nanos / 1_000_000 : 0);
        if (pvData.data.some(point => point.timestamp === timestamp)) return pvData;

        let newPvData = { ...pvData };
        newPvData.data = [...pvData.data, {
          timestamp,
          severity: newPoint.severity || 0,
          status: newPoint.status || 0,
          value,
          min: value,
          max: value,
          stddev: 0,
          count: 1
        }];

        if (s.liveModeConfig.mode === "rolling") {
          const cutoff = Date.now() - (s.timeRange.end.getTime() - s.timeRange.start.getTime());
          newPvData.data = newPvData.data.filter(point => point.timestamp >= cutoff);
        }

        return newPvData;
      });

      const now = new Date();
      return {
        data: newData,
        timeRange: s.liveModeConfig.mode === "rolling" 
          ? {
              start: new Date(now.getTime() - (s.timeRange.end.getTime() - s.timeRange.start.getTime())),
              end: now
            }
          : { ...s.timeRange, end: now },
        lastRefresh: now
      };
    });
  };

  const fetchDataForPVs = async () => {
    if (state.selectedPVs.length === 0) return;

    const timeRangeSeconds = Math.floor(
        (state.timeRange.end.getTime() - state.timeRange.start.getTime()) / 1000
    );

    console.log("Time range (seconds):", timeRangeSeconds);
    console.log("Using operator:", state.dataOperator);

    setState("loading", true);
    try {
        const data = await fetchData(
            state.selectedPVs.map(pv => pv.name),
            state.timeRange.start,
            state.timeRange.end,
            {
                timezone: state.timezone,
                mode: state.liveModeConfig.mode,
                operator: state.dataOperator,
                fetchLatestMetadata: true
            }
        );

        setState({
            data,
            error: null,
            isConnected: true,
            lastRefresh: new Date()
        });

        data.forEach(pvData => {
            if (pvData.meta) {
                updatePVMetadata(pvData.meta.name, pvData.meta);
            }
        });
    } catch (error) {
        console.error("Fetch error:", error);
        setState({
            error: String(error),
            isConnected: false
        });
    } finally {
        setState("loading", false);
    }
  };

  const toggleLiveMode = async () => {
    if (state.liveModeConfig.enabled) {
      console.log("Stopping live mode");
      await liveManager?.stop();
      setState("liveModeConfig", "enabled", false);
    } else {
      console.log("Starting live mode");
      liveManager = new LiveUpdateManager();
      await liveManager.start({
        pvs: state.selectedPVs.map(pv => pv.name),
        updateIntervalMs: state.liveModeConfig.updateInterval,
        timezone: state.timezone,
        onData: processLiveData
      });
      setState("liveModeConfig", "enabled", true);
    }
  };

  onMount(() => {
    clearStoredConfigs();
  });

  createEffect(() => {
    saveState("selectedPVs", state.selectedPVs);
    saveState("visiblePVs", Array.from(state.visiblePVs));
    saveState("timeRange", state.timeRange);
    saveState("timezone", state.timezone);
    saveState("liveModeConfig", state.liveModeConfig);
    saveState("axes", Array.from(state.axes.entries()));
    saveState("dataOperator", state.dataOperator);
    saveState("fetchOptions", state.fetchOptions);
  });

  onCleanup(() => {
    liveManager?.stop();
  });

  return (
    <ErrorBoundary fallback={(err) => <div>Error: {err.toString()}</div>}>
      <div class="grid grid-cols-[300px_1fr_300px] gap-4 p-4 bg-gray-50 min-h-screen">
        <div class="space-y-4">
          <PVSelector
            selectedPVs={() => state.selectedPVs}
            visiblePVs={() => state.visiblePVs}
            onAddPV={(pv, properties) => {
              setState("selectedPVs", pvs => [...pvs, { name: pv, pen: properties }]);
              setState("visiblePVs", pvs => new Set([...pvs, pv]));
              
              // Fetch metadata for the new PV
              getPVMetadata(pv).then(metadata => {
                const axisId = findOrCreateAxis(metadata);
                updatePVMetadata(pv, metadata);
              });

              fetchDataForPVs();
            }}
            onUpdatePV={(pv, properties) => {
              setState("selectedPVs", pvs => 
                pvs.map(p => p.name === pv ? { ...p, pen: properties } : p)
              );
            }}
            onRemovePV={(pv) => {
              setState(s => {
                const newPvs = s.selectedPVs.filter(p => p.name !== pv);
                const newVisible = new Set(s.visiblePVs);
                newVisible.delete(pv);
                
                // Clean up axes
                const axes = new Map(s.axes);
                for (const [id, axis] of axes) {
                  axis.pvs.delete(pv);
                  if (axis.pvs.size === 0) {
                    axes.delete(id);
                  }
                }
                
                return {
                  selectedPVs: newPvs,
                  visiblePVs: newVisible,
                  axes
                };
              });
            }}
            onVisibilityChange={(pv, isVisible) => {
              setState("visiblePVs", pvs => {
                const newPvs = new Set(pvs);
                isVisible ? newPvs.add(pv) : newPvs.delete(pv);
                return newPvs;
              });
            }}
          />
          <AxisManager
            axes={() => state.axes}
            onAxisChange={(assignment) => {
              setState("selectedPVs", pvs => {
                const pvIndex = pvs.findIndex(p => p.name === assignment.pvName);
                if (pvIndex === -1) return pvs;
          
                // Clean up old axis assignment
                const oldAxisId = pvs[pvIndex].axisId;
                if (oldAxisId) {
                  setState("axes", axes => {
                    const axis = axes.get(oldAxisId);
                    if (axis) {
                      axis.pvs.delete(assignment.pvName);
                      if (axis.pvs.size === 0) {
                        axes.delete(oldAxisId);
                      }
                    }
                    return axes;
                  });
                }
          
                // Update to new axis
                setState("axes", axes => {
                  const axis = axes.get(assignment.axisId);
                  if (axis) {
                    axis.pvs.add(assignment.pvName);
                    axis.autoRange = assignment.autoRange;
                    if (!assignment.autoRange && assignment.range) {
                      axis.range = {
                        low: assignment.range.low,
                        high: assignment.range.high
                      };
                    }
                  }
                  return axes;
                });
          
                pvs[pvIndex] = { ...pvs[pvIndex], axisId: assignment.axisId };
                return [...pvs];
              });
            }}
            onAxisEdit={(updatedAxis: AxisConfig) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                newAxes.set(updatedAxis.id, updatedAxis);
                return newAxes;
              });
            }}
            onAxisAdd={(newAxis: AxisConfig) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                newAxes.set(newAxis.id, newAxis);
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
            onExport={() => setState("showExportDialog", true)}
          />
          
          <div class="bg-white rounded-lg shadow-sm p-4">
            <div ref={chartContainer} class="w-full h-[calc(100vh-280px)] relative">
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
                  pvs={state.selectedPVs.filter(pv => state.visiblePVs.has(pv.name))}
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
                timezone
              });
              fetchDataForPVs();
            }}
            disabled={state.loading}
            initialTimezone={state.timezone}
            currentStartDate={state.timeRange.start}
            currentEndDate={state.timeRange.end}
          />
          <ConfigurationManager
            onSave={(name) => {
              const config = {
                selectedPVs: state.selectedPVs,
                visiblePVs: Array.from(state.visiblePVs),
                timeRange: state.timeRange,
                timezone: state.timezone,
                liveModeConfig: state.liveModeConfig,
                axes: Array.from(state.axes.entries()),
                dataOperator: state.dataOperator,
                fetchOptions: state.fetchOptions
              };
              saveState(name, config);
            }}
            onLoad={(name) => {
              const config = loadState(name);
              if (config) {
                setState({
                  ...config,
                  axes: new Map(config.axes),
                  visiblePVs: new Set(config.visiblePVs)
                });
                fetchDataForPVs();
              }
            }}
          />
        </div>

        <ConnectionStatus isConnected={state.isConnected} />
        
        <Show when={state.showDebugData}>
          <DebugDialog
            isOpen={true}
            onClose={() => setState("showDebugData", false)}
            data={state.debugLogs}
          />
        </Show>
      </div>
    </ErrorBoundary>
  );
}