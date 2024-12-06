import { createSignal, createEffect, createMemo, onMount, onCleanup, Show } from "solid-js";
import { createStore, StoreSetter } from "solid-js/store";
import { ErrorBoundary } from "solid-js";
import { fetchData, getPVMetadata, testConnection } from "../utils/archiverApi";
import { DataFormat, ProcessingMode, UPlotData, PVWithProperties, AxisConfig } from "../types";
import UnifiedManager from "../components/controls/UnifiedManager";
import ControlPanel from "../components/controls/ControlPanel";
import UPlotChart from "../components/chart/UPlotChart";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import ConnectionStatus from "../components/controls/ConnectionStatus";

const CONNECTION_CHECK_INTERVAL = 30000;
const DEFAULT_UPDATE_INTERVAL = 1000;

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

const INITIAL_STATE: ViewerState = {
  selectedPVs: [],
  visiblePVs: new Set(),
  timeRange: {
    start: new Date(Date.now() - 3600000),
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
};

export default function ArchiveViewer() {
  const [state, setState] = createStore<ViewerState>(INITIAL_STATE);
  const [liveUpdateInterval, setLiveUpdateInterval] = createSignal<number | null>(null);

  const handlePVMetadata = async (pvName: string, metadata: any) => {
    const egu = metadata.EGU || "Value";
    let axisId = `axis_${egu.toLowerCase().replace(/[^a-z0-9]/g, "_")}`;

    const displayLimits = {
      low: parseFloat(metadata.LOPR as string) || -100,
      high: parseFloat(metadata.HOPR as string) || 100
    };

    const existingAxis = Array.from(state.axes.values()).find(
      axis => axis.EGU.toLowerCase() === egu.toLowerCase()
    );

    if (existingAxis) {
      axisId = existingAxis.id;
      setState("axes", axes => {
        const newAxes = new Map(axes);
        const axis = newAxes.get(existingAxis.id);
        if (axis) {
          axis.pvs = new Set([...axis.pvs, pvName]);
          if (!axis.autoRange) {
            axis.range = {
              low: Math.min(axis.range?.low ?? Infinity, displayLimits.low),
              high: Math.max(axis.range?.high ?? -Infinity, displayLimits.high)
            };
          }
        }
        return newAxes;
      });
    } else {
      setState("axes", axes => {
        const newAxes = new Map(axes);
        newAxes.set(axisId, {
          id: axisId,
          EGU: egu,
          position: newAxes.size % 2 === 0 ? "left" : "right",
          autoRange: true,
          range: displayLimits,
          pvs: new Set([pvName])
        });
        return newAxes;
      });
    }

    setState("selectedPVs", pvs => 
      pvs.map(p => p.name === pvName ? { ...p, axisId } : p)
    );
  };

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
        lastRefresh: new Date() 
      });
    } catch (error) {
      console.error("Fetch error:", error);
      setState("error", String(error));
    } finally {
      setState("loading", false);
    }
  };

  const handleLiveUpdate = async () => {
    if (!state.selectedPVs.length || !state.data?.timestamps.length) return;
  
    const lastTimestamp = state.data.timestamps[state.data.timestamps.length - 1];
    const now = new Date();
  
    try {
      const data = await fetchData(
        state.selectedPVs.map(pv => pv.name),
        new Date(lastTimestamp + 1), // Add 1ms to avoid overlap
        now,
        ProcessingMode.Raw,
        state.dataFormat
      );
  
      if (!data.timestamps.length) return;
  
      if (state.liveModeConfig.mode === "rolling") {
        const cutoffTime = now.getTime() - state.timeRangeSeconds * 1000;
        
        // Combine and sort timestamps
        const allTimestamps = [...state.data.timestamps, ...data.timestamps]
          .filter(t => t >= cutoffTime)
          .sort((a, b) => a - b);
  
        // Create index maps for both datasets
        const oldIndexMap = new Map(state.data.timestamps.map((t, i) => [t, i]));
        const newIndexMap = new Map(data.timestamps.map((t, i) => [t, i]));
  
        const mergedData = {
          timestamps: allTimestamps,
          series: state.data.series.map((_, seriesIndex) => 
            allTimestamps.map(t => {
              if (t >= cutoffTime) {
                const oldIndex = oldIndexMap.get(t);
                const newIndex = newIndexMap.get(t);
                if (newIndex !== undefined) return data.series[seriesIndex][newIndex];
                if (oldIndex !== undefined) return state.data!.series[seriesIndex][oldIndex];
                return NaN;
              }
              return NaN;
            })
          ),
          meta: data.meta
        };
  
        setState({
          data: mergedData,
          timeRange: {
            start: new Date(cutoffTime),
            end: now
          }
        });
      } else {
        // Sort timestamps for append mode too
        const allTimestamps = [...state.data.timestamps, ...data.timestamps]
          .sort((a, b) => a - b);
  
        setState("data", {
          timestamps: allTimestamps,
          series: state.data.series.map((oldSeries, i) => {
            const newSeries = data.series[i] || [];
            return [...oldSeries, ...newSeries];
          }),
          meta: data.meta
        });
        setState("timeRange", "end", now);
      }
    } catch (error) {
      console.error("Live update error:", error);
    }
  };



  const startLiveUpdates = () => {
    stopLiveUpdates();
    // Start interval immediately
    const interval = setInterval(handleLiveUpdate, state.liveModeConfig.updateInterval);
    setLiveUpdateInterval(interval as unknown as number);
    // Trigger first update
    handleLiveUpdate();
  };
  const stopLiveUpdates = () => {
    console.log("Stopping live updates");
    const interval = liveUpdateInterval();
    if (interval) {
      clearInterval(interval);
      setLiveUpdateInterval(null);
    }
  };
  
  const checkConnection = async () => {
    try {
      const isConnected = await testConnection(state.dataFormat);
      setState("isConnected", isConnected);
    } catch (error) {
      setState("isConnected", false);
    }
  };

  createEffect(() => {
    if (state.liveModeConfig.enabled) {
      startLiveUpdates();
    } else {
      stopLiveUpdates();
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
            onAxisEdit={(updatedAxis: AxisConfig) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                const existing = newAxes.get(updatedAxis.id);
                if (existing) {
                  updatedAxis.pvs = existing.pvs;
                }
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
            onAxisRemove={(axisId: string) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                const axis = newAxes.get(axisId);
                if (axis && axis.pvs.size === 0) {
                  newAxes.delete(axisId);
                }
                return newAxes;
              });
            }}
            onAddPV={async (pv: string, properties: any) => {
              setState("selectedPVs", pvs => [...pvs, { name: pv, pen: properties }]);
              setState("visiblePVs", pvs => new Set([...pvs, pv]));
            
              try {
                const metadata = await getPVMetadata(pv);
                await handlePVMetadata(pv, metadata);
              } catch (error) {
                console.error(`Failed to fetch metadata for ${pv}:`, error);
                await handlePVMetadata(pv, {
                  name: pv,
                  EGU: "Value",
                  LOPR: "-100",
                  HOPR: "100"
                });
              }
              
              await fetchDataForPVs();
            }}
            onUpdatePV={(pv: string, properties: any, axisId: string) => {
              if (axisId) {
                const oldAxisId = state.selectedPVs.find(p => p.name === pv)?.axisId;
                setState("axes", axes => {
                  const newAxes = new Map(axes);
                  if (oldAxisId) {
                    const oldAxis = newAxes.get(oldAxisId);
                    if (oldAxis) oldAxis.pvs.delete(pv);
                  }
                  const newAxis = newAxes.get(axisId);
                  if (newAxis) newAxis.pvs.add(pv);
                  return newAxes;
                });
              }
              setState("selectedPVs", pvs =>
                pvs.map(p => p.name === pv ? { ...p, pen: properties, axisId } : p)
              );
            }}
            onRemovePV={(pv: string) => {
              const pvInfo = state.selectedPVs.find(p => p.name === pv);
              if (pvInfo?.axisId) {
                setState("axes", axes => {
                  const axis = axes.get(pvInfo.axisId!);
                  if (axis) axis.pvs.delete(pv);
                  return axes;
                });
              }
              setState("selectedPVs", pvs => pvs.filter(p => p.name !== pv));
              setState("visiblePVs", pvs => {
                const newPvs = new Set(pvs);
                newPvs.delete(pv);
                return newPvs;
              });
            }}
            onVisibilityChange={(pv: string, isVisible: any) => {
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
   setState("liveModeConfig", "enabled", !state.liveModeConfig.enabled);
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
                timeRangeSeconds: seconds
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
