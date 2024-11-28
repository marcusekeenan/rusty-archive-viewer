import { createSignal, createEffect, createMemo, onMount, onCleanup, Show } from "solid-js";
import { createStore } from "solid-js/store";
import { ErrorBoundary } from "solid-js";

import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import ControlPanel from "../components/controls/ControlPanel";
import ChartJS from "../components/chart/ChartJS";
import ConnectionStatus from "../components/controls/ConnectionStatus";
import UnifiedManager from "../components/controls/UnifiedManager";

import { fetchData, fetchLatest, getPVMetadata, testConnection } from "../utils/archiverApi";
import { PVWithProperties, PenProperties, AxisConfig, PVMetadata } from "../types/pv";
import { PVData, Point, Meta } from "../types/rust_types";
import { DataOperator } from "../types/api";
import { getCommonMetadata } from "../types/pv";

// Constants
const CONNECTION_CHECK_INTERVAL = 30000;
const DEFAULT_UPDATE_INTERVAL = 1000;
const MIN_UPDATE_INTERVAL = 1000;
const MAX_UPDATE_INTERVAL = 30000;

interface ViewerState {
  selectedPVs: PVWithProperties[];
  visiblePVs: Set<string>;
  timeRange: {
    start: Date;
    end: Date;
  };
  data: PVData[];
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
  const [state, setState] = createStore<ViewerState>(INITIAL_STATE);
  const [liveUpdateInterval, setLiveUpdateInterval] = createSignal<number | null>(null);

  // Memoized visible data
  const visibleData = createMemo(() => 
    state.data.filter(pv => state.visiblePVs.has(pv.meta.name))
  );

  // Reactive signals for components
  const selectedPVs = () => state.selectedPVs;
  const visiblePVs = () => state.visiblePVs;
  const axes = () => state.axes;

  const checkConnection = async () => {
    try {
      const isConnected = await testConnection();
      setState("isConnected", isConnected);
    } catch (error) {
      setState("isConnected", false);
      console.error("Connection check failed:", error);
    }
  };

  const updatePVMetadata = async (pvName: string, metadata: Meta) => {
    const pvMetadata = getCommonMetadata(metadata);
    const egu = pvMetadata.EGU || "Value";
    
    // Create display limits from metadata
    const displayLimits = {
      low: parseFloat(metadata.LOPR as string) || -100,
      high: parseFloat(metadata.HOPR as string) || 100
    };

    const existingAxis = Array.from(state.axes.values()).find(
      axis => axis.EGU.toLowerCase() === egu.toLowerCase()
    );

    let axisId: string;
    if (existingAxis) {
      axisId = existingAxis.id;
      setState("axes", axes => {
        const newAxes = new Map(axes);
        const axis = newAxes.get(axisId);
        if (axis) {
          axis.pvs.add(pvName);
        }
        return newAxes;
      });
    } else {
      axisId = `axis_${egu.toLowerCase().replace(/[^a-z0-9]/g, "_")}_${Date.now()}`;
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

    // Update PV metadata and axis assignment
    setState("selectedPVs", pvs => 
      pvs.map(pv => 
        pv.name === pvName
          ? { ...pv, metadata: pvMetadata as PVMetadata, axisId }
          : pv
      )
    );
  };

  const fetchDataForPVs = async () => {
    if (state.selectedPVs.length === 0) return;

    setState("loading", true);
    try {
      const data = await fetchData(
        state.selectedPVs.map(pv => pv.name),
        state.timeRange.start,
        state.timeRange.end
      );
      setState({ data, error: null, lastRefresh: new Date() });
    } catch (error) {
      console.error("Fetch error:", error);
      setState("error", String(error));
    } finally {
      setState("loading", false);
    }
  };

  const handleAxisAssignment = (pvName: string, newAxisId: string) => {
    // Remove PV from old axis
    setState("axes", axes => {
      const newAxes = new Map(axes);
      for (const [id, axis] of newAxes.entries()) {
        if (id !== newAxisId) {
          axis.pvs.delete(pvName);
        }
      }
      const newAxis = newAxes.get(newAxisId);
      if (newAxis) {
        newAxis.pvs.add(pvName);
      }
      return newAxes;
    });
  };

  onMount(() => {
    checkConnection();
    const interval = setInterval(checkConnection, CONNECTION_CHECK_INTERVAL);
    onCleanup(() => {
      clearInterval(interval);
      if (liveUpdateInterval()) {
        clearInterval(liveUpdateInterval()!);
      }
    });
  });

  createEffect(() => {
    if (!state.isConnected && state.liveModeConfig.enabled) {
      if (liveUpdateInterval()) {
        clearInterval(liveUpdateInterval()!);
        setLiveUpdateInterval(null);
      }
      setState("liveModeConfig", "enabled", false);
    }
  });

  return (
    <ErrorBoundary fallback={(err) => <div>Error: {err.toString()}</div>}>
      <div class="grid grid-cols-[350px_1fr_300px] gap-4 p-4 bg-gray-50 h-full overflow-hidden">
        <div class="overflow-auto">
          <UnifiedManager
            selectedPVs={selectedPVs}
            visiblePVs={visiblePVs}
            axes={axes}
            onAxisEdit={(updatedAxis) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                newAxes.set(updatedAxis.id, updatedAxis);
                return newAxes;
              });
            }}
            onAxisAdd={(newAxis) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                newAxes.set(newAxis.id, newAxis);
                return newAxes;
              });
            }}
            onAxisRemove={(axisId) => {
              setState("axes", axes => {
                const newAxes = new Map(axes);
                if (newAxes.get(axisId)?.pvs.size === 0) {
                  newAxes.delete(axisId);
                }
                return newAxes;
              });
            }}
            onAddPV={async (pv, properties) => {
              setState("selectedPVs", pvs => [
                ...pvs,
                { name: pv, pen: properties }
              ]);
              setState("visiblePVs", pvs => new Set([...pvs, pv]));

              try {
                const metadata = await getPVMetadata(pv);
                await updatePVMetadata(pv, metadata);
              } catch (error) {
                console.warn(`Failed to fetch metadata for ${pv}`, error);
                await updatePVMetadata(pv, {
                  name: pv,
                  EGU: "Value",
                  DESC: "No metadata available",
                  LOPR: "-100",
                  HOPR: "100"
                });
              }

              await fetchDataForPVs();
            }}
            onUpdatePV={(pv, properties, axisId) => {
              setState("selectedPVs", pvs =>
                pvs.map(p =>
                  p.name === pv ? { ...p, pen: properties, axisId } : p
                )
              );
              handleAxisAssignment(pv, axisId);
            }}
            onRemovePV={(pv) => {
              setState(s => {
                const newPvs = s.selectedPVs.filter(p => p.name !== pv);
                const newVisible = new Set(s.visiblePVs);
                newVisible.delete(pv);
                return { selectedPVs: newPvs, visiblePVs: newVisible };
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
        </div>

        <div class="flex flex-col gap-4">
          <ControlPanel
            liveModeConfig={() => state.liveModeConfig}
            dataOperator={() => state.dataOperator}
            onLiveModeToggle={() => {
              setState("liveModeConfig", "enabled", !state.liveModeConfig.enabled);
            }}
            onLiveModeConfigChange={(config) => {
              setState("liveModeConfig", prev => ({ ...prev, ...config }));
            }}
            onDataOperatorChange={(operator) => {
              setState("dataOperator", operator);
            }}
            onRefresh={fetchDataForPVs}
            onExport={() => {
              // Implement export functionality
            }}
            loading={() => state.loading}
          />

          <ChartJS
            data={visibleData()}
            timeRange={state.timeRange}
            pvs={state.selectedPVs}
            axes={state.axes}
          />
        </div>

        <div class="overflow-auto">
          <TimeRangeSelector
            initialTimezone={state.timezone}
            currentStartDate={state.timeRange.start}
            currentEndDate={state.timeRange.end}
            onChange={(start, end, timezone) => {
              setState("timeRange", { start, end });
              setState("timezone", timezone);
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