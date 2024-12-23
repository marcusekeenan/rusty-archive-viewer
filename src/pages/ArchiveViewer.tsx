import {
  createSignal,
  createEffect,
  onMount,
  onCleanup,
  Show,
} from "solid-js";
import { createStore } from "solid-js/store";
import { ErrorBoundary } from "solid-js";
import { fetchData, getPVMetadata, testConnection } from "../utils/archiverApi";
import {
  DataFormat,
  ProcessingMode,
  UPlotData,
  PVWithProperties,
  AxisConfig,
  PenProperties,
} from "../types";
import UnifiedManager from "../components/controls/UnifiedManager";
import ControlPanel from "../components/controls/ControlPanel";
import UPlotChart from "../components/chart/UPlotChart";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import ConnectionStatus from "../components/controls/ConnectionStatus";

// Constants
const CONNECTION_CHECK_INTERVAL = 30000;
const DEFAULT_UPDATE_INTERVAL = 1000;
const HOUR_IN_MS = 3600000;

// Types
interface ViewerState {
  selectedPVs: PVWithProperties[];
  visiblePVs: Set<string>;
  timeRange: {
    start: Date;
    end: Date;
  };
  timeRangeSeconds: number;
  data: UPlotData | null;
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
  processingMode: ProcessingMode;
  dataFormat: DataFormat;
  isConnected: boolean;
}

export default function ArchiveViewer() {
  // Initialize state with default values
  const [state, setState] = createStore<ViewerState>({
    selectedPVs: [],
    visiblePVs: new Set(),
    timeRange: {
      start: new Date(Date.now() - HOUR_IN_MS),
      end: new Date(),
    },
    timeRangeSeconds: 3600,
    data: null,
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
    processingMode: ProcessingMode.Optimized,
    dataFormat: DataFormat.Raw,
    isConnected: true,
  });

  // Live update handling
  const [liveUpdateInterval, setLiveUpdateInterval] = createSignal<number | null>(null);
  const [lastRequestTime, setLastRequestTime] = createSignal<number>(Date.now());
  const [isUpdating, setIsUpdating] = createSignal(false);

  // Core data fetching function
  const fetchDataForPVs = async () => {
    if (!state.selectedPVs.length) return;

    setState("loading", true);
    try {
      const data = await fetchData(
        state.selectedPVs.map(pv => pv.name),
        state.timeRange.start,
        state.timeRange.end,
        state.processingMode,
        state.dataFormat
      );

      setState({
        data,
        error: null,
        lastRefresh: new Date(),
      });
    } catch (error) {
      console.error("Fetch error:", error);
      setState("error", String(error));
    } finally {
      setState("loading", false);
    }
  };

  // Metadata handling
  const handlePVMetadata = async (pvName: string, metadata: any) => {
    const egu = metadata.EGU || "Value";
    const axisId = `axis_${egu.toLowerCase().replace(/[^a-z0-9]/g, "_")}`;

    setState("axes", axes => {
      const newAxes = new Map(axes);
      let axis = Array.from(newAxes.values()).find(a => a.EGU === egu);
      
      if (!axis) {
        axis = {
          id: axisId,
          EGU: egu,
          position: newAxes.size % 2 === 0 ? "left" : "right",
          autoRange: true,
          range: {
            low: parseFloat(metadata.LOPR) || -100,
            high: parseFloat(metadata.HOPR) || 100
          },
          pvs: new Set([pvName])
        };
        newAxes.set(axisId, axis);
      } else {
        axis.pvs.add(pvName);
      }
      return newAxes;
    });

    setState("selectedPVs", pvs => 
      pvs.map(p => p.name === pvName ? {
        ...p,
        axisId,
        metadata: { name: pvName, EGU: egu, ...metadata }
      } : p)
    );
  };

  // Live update handling
  const handleLiveUpdate = async () => {
    if (!state.selectedPVs.length || !state.data?.timestamps.length || isUpdating()) return;

    try {
      setIsUpdating(true);
      const now = Date.now();
      const data = await fetchData(
        state.selectedPVs.map(pv => pv.name),
        new Date(lastRequestTime()),
        new Date(now),
        ProcessingMode.Raw,
        state.dataFormat
      );

      setLastRequestTime(now);
      
      if (!data.timestamps.length) return;

      if (state.liveModeConfig.mode === "rolling") {
        const cutoffTime = now - state.timeRangeSeconds * 1000;
        
        setState("data", {
          timestamps: [...(state.data?.timestamps || []), ...data.timestamps]
            .filter(ts => ts >= cutoffTime),
          series: state.data!.series.map((series, i) => [
            ...series,
            ...(data.series[i] || [])
          ].filter((_, idx) => (state.data?.timestamps[idx] || 0) >= cutoffTime)),
          meta: data.meta
        });

        setState("timeRange", {
          start: new Date(cutoffTime),
          end: new Date(now)
        });
      } else {
        setState("data", {
          timestamps: [...(state.data?.timestamps || []), ...data.timestamps],
          series: state.data!.series.map((series, i) => [
            ...series,
            ...(data.series[i] || [])
          ]),
          meta: data.meta
        });

        setState("timeRange", "end", new Date(now));
      }
    } catch (error) {
      console.error("Live update error:", error);
    } finally {
      setIsUpdating(false);
    }
  };

  // Live mode controls
  const startLiveUpdates = () => {
    stopLiveUpdates();
    setLastRequestTime(Date.now());
    setState("liveModeConfig", "enabled", true);
  };

  const stopLiveUpdates = () => {
    const interval = liveUpdateInterval();
    if (interval) {
      window.clearInterval(interval);
      setLiveUpdateInterval(null);
    }
    setState("liveModeConfig", "enabled", false);
  };

  // Connection handling
  const checkConnection = async () => {
    try {
      const isConnected = await testConnection(state.dataFormat);
      setState("isConnected", isConnected);
    } catch (error) {
      setState("isConnected", false);
    }
  };

  // Effects
  createEffect(() => {
    if (state.liveModeConfig.enabled) {
      const interval = window.setInterval(handleLiveUpdate, DEFAULT_UPDATE_INTERVAL);
      setLiveUpdateInterval(interval);
    } else {
      const currentInterval = liveUpdateInterval();
      if (currentInterval !== null) {
        window.clearInterval(currentInterval);
        setLiveUpdateInterval(null);
      }
    }
  });

  onMount(() => {
    checkConnection();
    const interval = setInterval(checkConnection, CONNECTION_CHECK_INTERVAL);
    onCleanup(() => {
      clearInterval(interval);
      stopLiveUpdates();
    });
  });

  return (
    <ErrorBoundary fallback={(err) => <div>Error: {err.toString()}</div>}>
      <div class="grid grid-cols-[350px_1fr_300px] gap-4 p-4 bg-gray-50 h-full overflow-hidden">
        <div class="overflow-auto">
          <UnifiedManager
            selectedPVs={() => state.selectedPVs}
            visiblePVs={() => state.visiblePVs}
            axes={() => state.axes}
            onAxisEdit={(axis: AxisConfig) => 
              setState("axes", axes => {
                const newAxes = new Map(axes);
                const existing = newAxes.get(axis.id);
                if (existing) axis.pvs = existing.pvs;
                newAxes.set(axis.id, axis);
                return newAxes;
              })
            }
            onAxisAdd={(axis: AxisConfig) => 
              setState("axes", axes => {
                const newAxes = new Map(axes);
                axis.pvs = new Set();
                newAxes.set(axis.id, axis);
                return newAxes;
              })
            }
            onAxisRemove={(axisId: string) => 
              setState("axes", axes => {
                const newAxes = new Map(axes);
                const axis = newAxes.get(axisId);
                if (axis?.pvs.size === 0) newAxes.delete(axisId);
                return newAxes;
              })
            }
            onAddPV={async (pv: string, properties: PenProperties) => {
              // Update state atomically
              const initialUpdate = (state: { selectedPVs: any; visiblePVs: any; }) => ({
                ...state,
                selectedPVs: [...state.selectedPVs, { name: pv, pen: properties }],
                visiblePVs: new Set([...state.visiblePVs, pv])
              });
              setState(initialUpdate);
            
              try {
                // Get metadata and handle in one atomic update
                const metadata = await getPVMetadata(pv);
                const egu = metadata.EGU || "Value";
                const axisId = `axis_${egu.toLowerCase().replace(/[^a-z0-9]/g, "_")}`;
                
                setState(state => {
                  const newAxes = new Map(state.axes);
                  let axis = Array.from(newAxes.values()).find(a => a.EGU === egu);
                  
                  if (!axis) {
                    axis = {
                      id: axisId,
                      EGU: egu,
                      position: newAxes.size % 2 === 0 ? "left" : "right",
                      autoRange: true,
                      range: {
                        low: Number(metadata.LOPR ?? -100),
                        high: Number(metadata.HOPR ?? 100)
                      },
                      pvs: new Set([pv])
                    };
                    newAxes.set(axisId, axis);
                  } else {
                    axis.pvs.add(pv);
                  }
            
                  return {
                    ...state,
                    axes: newAxes,
                    selectedPVs: state.selectedPVs.map(p => 
                      p.name === pv ? { ...p, axisId, metadata } : p
                    )
                  };
                });
            
              } catch (error) {
                // Handle error case similarly
                console.error(`Failed to fetch metadata for ${pv}:`, error);
                // ... error handling
              }
            
              await fetchDataForPVs();
            }}
            onUpdatePV={(pv: string, properties: PenProperties, axisId: string) => {
              setState(s => {
                const oldAxisId = s.selectedPVs.find(p => p.name === pv)?.axisId;

                if (axisId && axisId !== oldAxisId) {
                  const newAxes = new Map(s.axes);
                  if (oldAxisId) {
                    const oldAxis = newAxes.get(oldAxisId);
                    if (oldAxis) oldAxis.pvs.delete(pv);
                  }
                  const newAxis = newAxes.get(axisId);
                  if (newAxis) newAxis.pvs.add(pv);
                  return {
                    ...s,
                    axes: newAxes,
                    selectedPVs: s.selectedPVs.map(p => 
                      p.name === pv ? { ...p, pen: properties, axisId } : p
                    )
                  };
                }

                return {
                  ...s,
                  selectedPVs: s.selectedPVs.map(p => 
                    p.name === pv ? { ...p, pen: properties, axisId } : p
                  )
                };
              });
            }}
            onRemovePV={(pv: string) => {
              setState(s => {
                const pvInfo = s.selectedPVs.find(p => p.name === pv);
                const newAxes = new Map(s.axes);
                
                if (pvInfo?.axisId) {
                  const axis = newAxes.get(pvInfo.axisId);
                  if (axis) axis.pvs.delete(pv);
                }

                const newVisiblePvs = new Set(s.visiblePVs);
                newVisiblePvs.delete(pv);

                return {
                  ...s,
                  axes: newAxes,
                  selectedPVs: s.selectedPVs.filter(p => p.name !== pv),
                  visiblePVs: newVisiblePvs
                };
              });
            }}
            onVisibilityChange={(pv: string, isVisible: boolean) => {
              setState("visiblePVs", pvs => {
                const newPvs = new Set(pvs);
                isVisible ? newPvs.add(pv) : newPvs.delete(pv);
                return newPvs;
              });
            }}
          />
        </div>

        <div class="flex flex-col gap-4">
          <ControlPanel
            liveModeConfig={() => state.liveModeConfig}
            processingMode={() => state.processingMode}
            onLiveModeToggle={() => {
              if (!state.liveModeConfig.enabled) {
                startLiveUpdates();
              } else {
                stopLiveUpdates();
              }
            }}
            onLiveModeConfigChange={(config) => {
              setState("liveModeConfig", prev => ({ ...prev, ...config }));
            }}
            onProcessingModeChange={(mode: ProcessingMode) => {
              setState("processingMode", mode);
              if (!state.liveModeConfig.enabled) {
                fetchDataForPVs();
              }
            }}
            onRefresh={fetchDataForPVs}
            loading={() => state.loading}
          />

          <div class="chart-container relative w-full h-[calc(100vh-200px)]">
            <Show when={state.data}>
              <UPlotChart
                data={state.data!}
                timeRange={state.timeRange}
                pvs={state.selectedPVs.filter(pv => state.visiblePVs.has(pv.name))}
                axes={state.axes}
              />
            </Show>
          </div>
        </div>

        <div class="overflow-auto">
          <TimeRangeSelector
            initialTimezone={state.timezone}
            currentStartDate={state.timeRange.start}
            currentEndDate={state.timeRange.end}
            onChange={(start: Date, end: Date, timezone: string) => {
              const seconds = (end.getTime() - start.getTime()) / 1000;
              setState(s => ({
                ...s,
                timeRange: { start, end },
                timezone,
                timeRangeSeconds: seconds,
              }));
              if (!state.liveModeConfig.enabled) {
                fetchDataForPVs();
              }
            }}
            isLiveMode={state.liveModeConfig.enabled}
            liveMode={state.liveModeConfig.mode}
          />
        </div>

        <ConnectionStatus isConnected={state.isConnected} />
      </div>
    </ErrorBoundary>
  );
}