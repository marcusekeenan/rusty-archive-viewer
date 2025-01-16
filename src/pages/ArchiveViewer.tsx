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
  EPICSData,
  PVWithProperties,
  AxisConfig,
  PenProperties,
} from "../types";
import UnifiedManager from "../components/controls/UnifiedManager";
import ControlPanel from "../components/controls/ControlPanel";
import EPICSChart from "../components/chart/EPICSChart";
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
  data: EPICSData | null;
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
  // Initialize state
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

  // Live update state
  const [liveUpdateInterval, setLiveUpdateInterval] = createSignal<number | null>(null);
  const [lastRequestTime, setLastRequestTime] = createSignal<number>(Date.now());
  const [isUpdating, setIsUpdating] = createSignal(false);
  const [autoRanges, setAutoRanges] = createSignal<Map<string, { low: number; high: number }>>(new Map());

  // Core data fetching
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

  // Handle auto range updates from chart
  const handleAxisRangeUpdate = (axisId: string, range: { low: number; high: number }) => {
    setAutoRanges(prev => {
      const newRanges = new Map(prev);
      newRanges.set(axisId, range);
      return newRanges;
    });
  };

  // Live update handling
  const handleLiveUpdate = async () => {
    if (!state.liveModeConfig.enabled) return;
   
    try {
      const now = Date.now();
      const data = await fetchData(
        state.selectedPVs.map(pv => pv.name),
        new Date(now - 2000),
        new Date(now),
        ProcessingMode.Raw,
        state.dataFormat
      );
   
      if (!data.timestamps.length) return;
   
      const latestData = {
        timestamp: data.timestamps[data.timestamps.length - 1],
        values: data.series.map((s: string | any[]) => s[s.length - 1])
      };
   
      if (state.liveModeConfig.mode === "rolling" && state.data) {
        const cutoffTime = now - state.timeRangeSeconds * 1000;
        const keptTimestamps = state.data.timestamps.filter(ts => ts >= cutoffTime);
        const keptSeriesData = state.data.series.map(series => 
          series.slice(-keptTimestamps.length)
        );
   
        setState("data", {
          ...state.data,
          timestamps: [...keptTimestamps, latestData.timestamp],
          series: state.data.series.map((_, i) => [
            ...keptSeriesData[i],
            latestData.values[i]
          ])
        });
      } else if (state.data) {
        setState("data", {
          ...state.data,
          timestamps: [...state.data.timestamps, latestData.timestamp], 
          series: state.data.series.map((oldSeries, i) => [
            ...oldSeries,
            latestData.values[i]
          ])
        });
      }
   
      setLastRequestTime(now);
    } catch (error) {
      console.error("Live update error:", error);
    }
  };

  const startLiveUpdates = () => {
    stopLiveUpdates();
    if (state.data?.timestamps.length) {
      setLastRequestTime(Math.max(...state.data.timestamps));
    } else {
      setLastRequestTime(Date.now() - 2000);
    }
    setState("liveModeConfig", "enabled", true);
  };

  const stopLiveUpdates = () => {
    const interval = liveUpdateInterval();
    if (interval) {
      clearInterval(interval);
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
      const interval = setInterval(handleLiveUpdate, DEFAULT_UPDATE_INTERVAL);
      setLiveUpdateInterval(interval as unknown as number);
    } else {
      const interval = liveUpdateInterval();
      if (interval) {
        clearInterval(interval);
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
            autoRanges={autoRanges}
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
              setState(state => ({
                ...state,
                selectedPVs: [...state.selectedPVs, { name: pv, pen: properties }],
                visiblePVs: new Set([...state.visiblePVs, pv])
              }));
            
              try {
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
                console.error(`Failed to fetch metadata for ${pv}:`, error);
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
            <EPICSChart
              data={state.data!}
              timeRange={state.timeRange}
              pvs={state.selectedPVs.filter(pv => state.visiblePVs.has(pv.name))}
              axes={state.axes}
              timezone={state.timezone}  // Add this
              onAxisRangeUpdate={handleAxisRangeUpdate}
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