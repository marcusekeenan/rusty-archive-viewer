import { invoke } from "@tauri-apps/api/tauri";
import { emit } from '@tauri-apps/api/event';
import {
  createSignal,
  createEffect,
  createMemo,
  onCleanup,
  onMount,
  Show,
  For,
} from "solid-js";
import DebugDialog from "./DebugDialog";
import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import LiveModeControls from "../components/controls/LiveModeControls";
import ChartJS from "../components/chart/ChartJS";
import ChartuPlot from "../components/chart/ChartuPlot";
import {
  fetchData,
  type NormalizedPVData,
  type PointValue,
  LiveUpdateManager
} from "../utils/archiverApi";
import type {
  PVWithProperties,
  PenProperties,
  AxisConfig,
  PVMetadata,
  AxisAssignment
} from "../components/controls/types";
import { Dynamic } from "solid-js/web";

// Types
interface TimeRange {
  start: Date;
  end: Date;
}

interface LiveModeConfig {
  enabled: boolean;
  mode: 'rolling' | 'append';
  updateInterval: number;
}

interface DebugLog {
  timestamp: string;
  message: string;
  type: "info" | "error" | "debug" | "success";
  details?: string | null;
}

const DEFAULT_UPDATE_INTERVAL = 1000; // 1 second

export default function ArchiveViewer() {
  let chartContainer: HTMLDivElement | undefined;
  let liveManager: LiveUpdateManager | undefined;

  // State Management
  const [selectedPVs, setSelectedPVs] = createSignal<PVWithProperties[]>([]);
  const [visiblePVs, setVisiblePVs] = createSignal<Set<string>>(new Set());
  const [timeRange, setTimeRange] = createSignal<TimeRange>({
    start: new Date(Date.now() - 3600000),
    end: new Date(),
  });
  const [data, setData] = createSignal<NormalizedPVData[]>([]);
  const [loading, setLoading] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);
  const [debugLogs, setDebugLogs] = createSignal<DebugLog[]>([]);
  const [showDebugData, setShowDebugData] = createSignal<boolean>(false);
  const [lastRefresh, setLastRefresh] = createSignal<Date | null>(null);
  const [selectedChart, setSelectedChart] = createSignal<"chartjs" | "uplot">("chartjs");
  const [timezone, setTimezone] = createSignal(
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  
  // Live Mode Configuration
  const [liveModeConfig, setLiveModeConfig] = createSignal<LiveModeConfig>({
    enabled: false,
    mode: 'rolling',
    updateInterval: DEFAULT_UPDATE_INTERVAL
  });

  // Axis Management
  const [axes, setAxes] = createSignal<Map<string, AxisConfig>>(new Map());

  // Computed values
  const chartWidth = createMemo(() => chartContainer?.clientWidth || window.innerWidth);
  
  const visibleData = createMemo(() => {
    // Add debug logging
    console.log("Calculating visibleData", {
      allData: data(),
      visibleSet: Array.from(visiblePVs()),
    });
    
    return data().filter((pv) => {
      // Explicitly check if the PV name is in the visible set
      const isVisible = visiblePVs().has(pv.meta.name);
      console.log(`PV ${pv.meta.name} visibility:`, isVisible);
      return isVisible;
    });
  });
  

  const totalPoints = createMemo(() => {
    return visibleData().reduce((sum, pv) => sum + (pv.data?.length || 0), 0);
  });

  // Helper Functions
  const generateAxisId = (egu: string) => {
    const base = egu.toLowerCase().replace(/[^a-z0-9]/g, '_');
    const existing = Array.from(axes().keys()).filter(k => k.startsWith(base));
    return existing.length ? `${base}_${existing.length + 1}` : base;
  };

  const updatePVMetadata = (pvName: string, metadata: PVMetadata) => {
    console.log("Updating metadata for", pvName, metadata);
    setSelectedPVs(prev => {
      const pvs = [...prev];
      const pvIndex = pvs.findIndex(pv => pv.name === pvName);
      if (pvIndex === -1) return prev;

      const pv = pvs[pvIndex];
      pv.metadata = metadata;

      // Find or create appropriate axis
      let axisId = pv.axisId;
      if (!axisId) {
        // Look for existing axis with same EGU
        const existingAxis = Array.from(axes().values())
          .find(axis => axis.egu.toLowerCase() === metadata.egu.toLowerCase());

        if (existingAxis) {
          axisId = existingAxis.id;
        } else {
          // Create new axis
          axisId = generateAxisId(metadata.egu);
          const newAxis: AxisConfig = {
            id: axisId,
            egu: metadata.egu,
            position: axes().size === 0 ? 'left' : 'right',
            autoRange: true,
            pvs: new Set([pvName])
          };
          
          setAxes(prev => {
            const next = new Map(prev);
            next.set(axisId, newAxis);
            return next;
          });
        }
      }

      pv.axisId = axisId;
      
      // Update axis PV set
      setAxes(prev => {
        const next = new Map(prev);
        const axis = next.get(axisId!);
        if (axis) {
          axis.pvs.add(pvName);
          if (axis.autoRange && metadata.displayLimits) {
            axis.range = metadata.displayLimits;
          }
        }
        return next;
      });

      return pvs;
    });
  };

  // Debug Logging
  const addDebugLog = async (
    message: string,
    type: DebugLog["type"] = "info",
    details: unknown = null
  ) => {
    const log: DebugLog = {
      timestamp: new Date().toISOString(),
      message,
      type,
      details: details ? JSON.stringify(details, null, 2) : null,
    };

    setDebugLogs(prev => [...prev.slice(-49), log]);

    if (type === "error") console.error(message, details);
    if (type === "debug") console.debug(message, details);

    try {
      await emit('debug-log', log);
    } catch (error) {
      console.debug('Failed to emit debug log:', error);
    }
  };

  // Live Data Processing
  const processLiveData = (pointValues: Record<string, PointValue>) => {
    console.log("Processing live data update:", pointValues);
    
    const config = liveModeConfig();
    const currentRange = timeRange();
    
    setData((prev) => {
      if (!prev?.length) return prev;

      return prev.map(pvData => {
        const pvName = pvData.meta.name;
        const newPoint = pointValues[pvName];

        if (!newPoint) {
          console.debug(`No new data for ${pvName}`);
          return pvData;
        }

        // Update metadata if available
        if (pvData.meta) {
          updatePVMetadata(pvName, pvData.meta);
        }

        const value = typeof newPoint.val === 'number' 
          ? newPoint.val 
          : Array.isArray(newPoint.val) 
            ? newPoint.val[0] 
            : null;

        if (value === null) {
          console.warn(`Invalid value format for ${pvName}:`, newPoint.val);
          return pvData;
        }

        const timestamp = newPoint.secs * 1000 + (newPoint.nanos ? newPoint.nanos / 1_000_000 : 0);

        // Check for duplicate timestamp
        if (pvData.data.some(point => point.timestamp === timestamp)) {
          console.debug(`Skipping duplicate timestamp for ${pvName}: ${timestamp}`);
          return pvData;
        }

        let updatedData = [...pvData.data, {
          timestamp,
          severity: newPoint.severity || 0,
          status: newPoint.status || 0,
          value,
          min: value,
          max: value,
          stddev: 0,
          count: 1
        }].sort((a, b) => a.timestamp - b.timestamp);

        // Handle rolling window
        if (config.mode === 'rolling') {
          const windowDuration = currentRange.end.getTime() - currentRange.start.getTime();
          const cutoffTime = Date.now() - windowDuration;
          updatedData = updatedData.filter(point => point.timestamp >= cutoffTime);
        }

        return {
          ...pvData,
          data: updatedData
        };
      });
    });

    // Update time range
    const now = new Date();
    if (config.mode === 'rolling') {
      const currentDuration = currentRange.end.getTime() - currentRange.start.getTime();
      setTimeRange({
        start: new Date(now.getTime() - currentDuration),
        end: now
      });
    } else {
      // Append mode - keep start time fixed
      setTimeRange(prev => ({
        start: prev.start,
        end: now
      }));
    }

    setLastRefresh(now);
  };

  // Data Fetching Logic
  const fetchDataForPVs = async () => {
    try {
      const pvs = selectedPVs();
      if (pvs.length === 0) {
        throw new Error("No PVs selected");
      }

      const range = timeRange();
      if (!range.start || !range.end) {
        throw new Error("Invalid time range");
      }

      const width = chartWidth();
      const responseData = await fetchData(
        pvs.map((pv) => pv.name),
        range.start,
        range.end,
        width,
        timezone()
      );

      if (Array.isArray(responseData) && responseData.length > 0) {
        // Process metadata first
        responseData.forEach(pvData => {
          if (pvData.meta) {
            updatePVMetadata(pvData.meta.name, pvData.meta);
          }
        });

        const dataWithProps = responseData.map((data, index) => ({
          ...data,
          pen: pvs[index].pen,
        }));

        setData(dataWithProps);
        setError(null);
        setLastRefresh(new Date());

        addDebugLog("Data fetched successfully", "debug", {
          timestamp: new Date().toISOString(),
          pointCount: dataWithProps.reduce((sum, pv) => sum + pv.data.length, 0),
          timeRange: {
            start: range.start.toISOString(),
            end: range.end.toISOString(),
          },
          timezone: timezone(),
        });
      } else {
        throw new Error("No data received");
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      addDebugLog(errorMessage, "error", { error: err });
    }
  };

  // Event Handlers
  const handleRefresh = async () => {
    setLoading(true);
    await fetchDataForPVs();
    setLoading(false);
  };

  const handleTimeRangeChange = (start: Date, end: Date, newTimezone: string, mode?: string) => {
    setTimezone(newTimezone);
    
    const isLive = liveModeConfig().enabled;
    if (isLive) {
      // In live mode, only update start time and timezone
      setTimeRange(prev => ({
        start: start,
        end: new Date()  // Always use current time for end
      }));
      
      // Re-fetch historical data with new range
      handleRefresh();
    } else {
      // Normal historical mode
      setTimeRange({ start, end });
      handleRefresh();
    }
  };

  const handleAddPV = (pv: string, properties: PenProperties) => {
    setSelectedPVs((prev) => [...prev, { 
      name: pv, 
      pen: properties 
      // axisId will be assigned when metadata arrives
    }]);
    setVisiblePVs((prev) => {
      const newSet = new Set(prev);
      newSet.add(pv);
      return newSet;
    });
    addDebugLog(`Added PV: ${pv}`, "info");
    handleRefresh();
  };

  const handleUpdatePV = (pv: string, properties: PenProperties) => {
    setSelectedPVs((prev) =>
      prev.map((p) => (p.name === pv ? { ...p, pen: properties } : p))
    );
    addDebugLog(`Updated PV properties: ${pv}`, "info");
    handleRefresh();
  };

  const handleRemovePV = (pv: string) => {
    setSelectedPVs((prev) => {
      const pvToRemove = prev.find(p => p.name === pv);
      if (pvToRemove?.axisId) {
        setAxes(axesPrev => {
          const next = new Map(axesPrev);
          const axis = next.get(pvToRemove.axisId!);
          if (axis) {
            axis.pvs.delete(pv);
            if (axis.pvs.size === 0) {
              next.delete(pvToRemove.axisId!);
            }
          }
          return next;
        });
      }
      return prev.filter((p) => p.name !== pv);
    });
    setVisiblePVs((prev) => {
      const newSet = new Set(prev);
      newSet.delete(pv);
      return newSet;
    });
    setData((prev) => prev.filter((p) => p.meta.name !== pv));
    addDebugLog(`Removed PV: ${pv}`, "info");
  };

  const handleVisibilityChange = (pv: string, isVisible: boolean) => {
    console.log("Visibility change", { pv, isVisible });
    setVisiblePVs((prev) => {
      const newSet = new Set(prev);
      if (isVisible) {
        newSet.add(pv);
      } else {
        newSet.delete(pv);
      }
      // Force reactivity by creating a new Set
      return new Set(newSet);
    });
  };

  const handleAxisAssignment = (assignment: AxisAssignment) => {
    setSelectedPVs(prev => {
      const pvs = [...prev];
      const pvIndex = pvs.findIndex(pv => pv.name === assignment.pvName);
      if (pvIndex === -1) return prev;

      const pv = pvs[pvIndex];
      const oldAxisId = pv.axisId;
      pv.axisId = assignment.axisId;

      setAxes(axesPrev => {
        const next = new Map(axesPrev);
        
        // Remove from old axis
        if (oldAxisId) {
          const oldAxis = next.get(oldAxisId);
          if (oldAxis) {
            oldAxis.pvs.delete(assignment.pvName);
            if (oldAxis.pvs.size === 0) {
              next.delete(oldAxisId);
            }
          }
        }

        // Add to new axis
        const newAxis = next.get(assignment.axisId);
        if (newAxis) {
          newAxis.pvs.add(assignment.pvName);
          newAxis.autoRange = assignment.autoRange;
          if (!assignment.autoRange && assignment.range) {
            newAxis.range = assignment.range;
          }
        }

        return next;
      });

      return pvs;
    });
  };

  const handleLiveModeConfigChange = async (newConfig: Partial<LiveModeConfig>) => {
    setLiveModeConfig(prev => ({ ...prev, ...newConfig }));
    
    if (newConfig.mode && newConfig.mode !== liveModeConfig().mode) {
      // Mode changed, re-fetch data with appropriate range
      const now = new Date();
      const currentRange = timeRange();
      
      if (newConfig.mode === 'rolling') {
        const duration = currentRange.end.getTime() - currentRange.start.getTime();
        setTimeRange({
          start: new Date(now.getTime() - duration),
          end: now
        });
      }
      
      await handleRefresh();
    }
  };

  const handleLiveModeToggle = async () => {
    const config = liveModeConfig();
    const newEnabled = !config.enabled;
    
    try {
      if (newEnabled) {
        const pvs = selectedPVs();
        if (!pvs.length) {
          throw new Error("No PVs selected");
        }

        // Create new manager
        liveManager = new LiveUpdateManager();
        
        setLiveModeConfig(prev => ({ ...prev, enabled: true }));
        
        // Initial data fetch based on mode
        const now = new Date();
        const currentRange = timeRange();
        const startTime = config.mode === 'rolling' 
          ? new Date(now.getTime() - (currentRange.end.getTime() - currentRange.start.getTime()))
          : currentRange.start;

        const initialData = await fetchData(
          pvs.map(pv => pv.name),
          startTime,
          now,
          chartWidth(),
          timezone()
        );

        if (!initialData?.length) {
          throw new Error("Failed to fetch initial data");
        }

        // Process metadata from initial data
        initialData.forEach(pvData => {
          if (pvData.meta) {
            updatePVMetadata(pvData.meta.name, pvData.meta);
          }
        });

        setData(initialData.map((pvData, index) => ({
          ...pvData,
          pen: pvs[index].pen
        })));

        // Start live updates
        await liveManager.start({
          pvs: pvs.map(pv => pv.name),
          updateIntervalMs: config.updateInterval,
          timezone: timezone(),
          onData: processLiveData
        });
      } else {
        // Stop live mode
        setLiveModeConfig(prev => ({ ...prev, enabled: false }));
        if (liveManager) {
          await liveManager.stop();
          liveManager = undefined;
        }

        // Fetch final state
        await handleRefresh();
      }
    } catch (error) {
      console.error("Live mode error:", error);
      setLiveModeConfig(prev => ({ ...prev, enabled: false }));
      if (liveManager) {
        await liveManager.stop();
        liveManager = undefined;
      }
      setError(error instanceof Error ? error.message : String(error));
    }
  };

  // Cleanup
  onCleanup(async () => {
    if (liveManager) {
      await liveManager.stop();
    }
  });

  return (
    <div class="p-4 bg-gray-50 min-h-screen">
      <div class="grid grid-cols-[300px_1fr_300px] gap-4">
        {/* Left Panel - PV Selection */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow-sm p-4">
            <div class="flex justify-between items-center mb-4">
              <h2 class="text-lg font-semibold">Variables</h2>
              <span class="text-sm text-gray-500">
                {selectedPVs().length} selected
              </span>
            </div>
            <PVSelector
              selectedPVs={selectedPVs}
              visiblePVs={visiblePVs}
              onAddPV={handleAddPV}
              onUpdatePV={handleUpdatePV}
              onRemovePV={handleRemovePV}
              onVisibilityChange={handleVisibilityChange}
            />
          </div>
        </div>

        {/* Center Panel - Chart and Controls */}
        <div class="space-y-4">
          {/* Control Bar */}
          <div class="bg-white rounded-lg shadow-sm p-4">
          <div class="flex items-center justify-between">
  {/* Chart Type Selection */}
  <div class="flex gap-4 items-center">
    <button
      class={`px-3 py-1.5 text-sm font-medium rounded-md ${
        selectedChart() === "chartjs"
          ? "bg-blue-50 text-blue-700"
          : "text-gray-600 hover:bg-gray-50"
      }`}
      onClick={() => setSelectedChart("chartjs")}
    >
      Chart.js
    </button>
    <button
      class={`px-3 py-1.5 text-sm font-medium rounded-md ${
        selectedChart() === "uplot"
          ? "bg-blue-50 text-blue-700"
          : "text-gray-600 hover:bg-gray-50"
      }`}
      onClick={() => setSelectedChart("uplot")}
    >
      µPlot
    </button>
  </div>

  {/* Live Mode Controls and Action Buttons */}
  <div class="flex items-center gap-4 ml-auto">
    {/* Live Mode Options - Show to the left */}
    <Show when={liveModeConfig().enabled}>
      <select
        value={liveModeConfig().mode}
        onChange={(e) => handleLiveModeConfigChange({ mode: e.target.value as 'rolling' | 'append' })}
        class="px-3 py-1.5 border rounded text-sm"
      >
        <option value="rolling">Rolling Window</option>
        <option value="append">Append</option>
      </select>
    </Show>

    {/* Live Toggle and Refresh Buttons - Always in the same place */}
    <div class="flex items-center gap-2">
      <button
        onClick={handleLiveModeToggle}
        class={`inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium 
          ${liveModeConfig().enabled
            ? "bg-red-100 text-red-700 hover:bg-red-200"
            : "bg-green-100 text-green-700 hover:bg-green-200"
          } transition-colors`}
      >
        {liveModeConfig().enabled ? (
          <>
            <div class="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
            <span>Live</span>
          </>
        ) : (
          <span>Go Live</span>
        )}
      </button>

      <Show when={!liveModeConfig().enabled}>
        <button
          onClick={handleRefresh}
          disabled={loading()}
          class="inline-flex items-center gap-2 px-3 py-1.5 bg-blue-50 text-blue-700 
                 rounded-md text-sm font-medium hover:bg-blue-100 
                 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {loading() ? (
            <>
              <div class="w-4 h-4 border-2 border-blue-700 border-t-transparent rounded-full animate-spin" />
              <span>Loading...</span>
            </>
          ) : (
            <>
              <span>Refresh</span>
              <span class="text-xs bg-blue-100 px-2 py-0.5 rounded-full">
                {totalPoints().toLocaleString()}
              </span>
            </>
          )}
        </button>
      </Show>
    </div>
  </div>
</div>

            {/* Status Bar */}
            <div class="mt-4 pt-4 border-t flex items-center justify-between text-sm text-gray-500">
              <div>
                {lastRefresh() && (
                  <span>
                    Last updated: {lastRefresh()?.toLocaleTimeString()}
                  </span>
                )}
              </div>
              <div>
                {error() && <span class="text-red-600">Error: {error()}</span>}
              </div>
            </div>
          </div>

          {/* Chart Area */}
          <div class="bg-white rounded-lg shadow-sm p-4">
            <div
              ref={chartContainer}
              class="w-full h-[calc(100vh-280px)] relative"
            >
              {data().length > 0 ? (
                <Show
                  when={selectedChart() === "chartjs"}
                  fallback={
                    <Dynamic
                      component={ChartuPlot}
                      data={visibleData()}
                      pvs={selectedPVs().filter(pv => visiblePVs().has(pv.name))}
                      timeRange={timeRange()}
                      timezone={timezone()}
                      axes={axes()}
                      onAxisChange={handleAxisAssignment}
                    />
                  }
                >
                  <Dynamic
                    component={ChartJS}
                    data={visibleData()}
                    pvs={selectedPVs().filter(pv => visiblePVs().has(pv.name))}
                    timeRange={timeRange()}
                    timezone={timezone()}
                    axes={axes()}
                    onAxisChange={handleAxisAssignment}
                  />
                </Show>
              ) : (
                <div class="absolute inset-0 flex items-center justify-center text-gray-400">
                  No data to display
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right Panel - Time Range */}
        <div class="bg-white rounded-lg shadow-sm p-4">
          <div class="flex justify-between items-center mb-4">
            <h2 class="text-lg font-semibold">Time Range</h2>
            <span class="text-sm text-gray-500">
              {liveModeConfig().enabled 
                ? `Live - ${liveModeConfig().mode === 'rolling' ? 'Rolling Window' : 'Append Mode'}` 
                : 'Historical'}
            </span>
          </div>
          <TimeRangeSelector
            onChange={handleTimeRangeChange}
            disabled={loading()}
            initialTimezone={timezone()}
            currentStartDate={timeRange().start}
            currentEndDate={timeRange().end}
          />
        </div>
      </div>

      {/* Debug Dialog */}
      <Show when={showDebugData()}>
        <DebugDialog
          isOpen={true}
          onClose={() => setShowDebugData(false)}
          data={debugLogs()}
        />
      </Show>
    </div>
  );
}   