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
} from "../components/controls/types";
import { Dynamic } from "solid-js/web";

// Types
interface TimeRange {
  start: Date;
  end: Date;
}

interface RealTimeConfig {
  enabled: boolean;
  bufferSize: number;
  updateInterval: number;
}

interface DebugLog {
  timestamp: string;
  message: string;
  type: "info" | "error" | "debug" | "success";
  details?: string | null;
}

interface DebugDialogProps {
  isOpen: boolean;
  onClose: () => void;
  data: DebugLog[];
}

const DEBUG_LOG_LIMIT = 50;
const DEFAULT_UPDATE_INTERVAL = 1000; // 1 second
const DEFAULT_BUFFER_SIZE = 3600; // 1 hour of data points

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
  const [realTimeConfig, setRealTimeConfig] = createSignal<RealTimeConfig>({
    enabled: false,
    bufferSize: DEFAULT_BUFFER_SIZE,
    updateInterval: DEFAULT_UPDATE_INTERVAL,
  });
  const [timezone, setTimezone] = createSignal(
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  const [timeRangeMode, setTimeRangeMode] = createSignal<"15min" | "hour" | "day" | "week" | "custom">("hour");

  // Computed values
  const chartWidth = createMemo(() => chartContainer?.clientWidth || window.innerWidth);
  const visibleData = createMemo(() => {
    const allData = data();
    const visiblePVNames = visiblePVs();
    return allData.filter((pv) => visiblePVNames.has(pv.meta.name));
  });

  const totalPoints = createMemo(() => {
    return visibleData().reduce((sum, pv) => sum + (pv.data?.length || 0), 0);
  });

  const getTimeRangeDuration = (mode: string): number => {
    switch (mode) {
        case "15min": return 15 * 60 * 1000;
        case "hour": return 3600 * 1000;
        case "day": return 24 * 3600 * 1000;
        case "week": return 7 * 24 * 3600 * 1000;
        default: return 3600 * 1000; // default to 1 hour
    }
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

    if (type === "error") console.error(message, details);
    if (type === "debug") console.debug(message, details);

    try {
      await emit('debug-log', log);
    } catch (error) {
      console.debug('Failed to emit debug log:', error);
    }
  };

  // Data Fetching Logic
  const fetchDataForPVs = async () => {
    console.log("the timezone is", timezone())
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
    if (mode) {
        setTimeRangeMode(mode as "15min" | "hour" | "day" | "week" | "custom");
    }
    setTimeRange({ start, end });
    setTimezone(newTimezone);
    handleRefresh();
};

  const handleAddPV = (pv: string, properties: PenProperties) => {
    setSelectedPVs((prev) => [...prev, { name: pv, pen: properties }]);
    setVisiblePVs((prev) => new Set(prev).add(pv));
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
    setSelectedPVs((prev) => prev.filter((p) => p.name !== pv));
    setVisiblePVs((prev) => {
      const newSet = new Set(prev);
      newSet.delete(pv);
      return newSet;
    });
    addDebugLog(`Removed PV: ${pv}`, "info");
    handleRefresh();
  };

  const handleRealTimeToggle = async () => {
    const config = realTimeConfig();
    const newEnabled = !config.enabled;
    
    setRealTimeConfig((prev) => ({ ...prev, enabled: newEnabled }));

    if (newEnabled) {
        liveManager = new LiveUpdateManager();
        try {
            const now = new Date();
            const duration = getTimeRangeDuration(timeRangeMode());
            const startTime = new Date(now.getTime() - duration);
            
            addDebugLog("Starting live mode", "info", {
                mode: timeRangeMode(),
                start: startTime.toISOString(),
                end: now.toISOString(),
                interval: config.updateInterval
            });
            
            setTimeRange({
                start: startTime,
                end: now
            });

            // First fetch to populate initial data
            await handleRefresh();

            await liveManager.start({
                pvs: selectedPVs().map(pv => pv.name),
                updateIntervalMs: config.updateInterval,
                timezone: timezone(),
                onData: (pointValues) => {
                    addDebugLog("Received live data update", "debug", {
                        timestamp: new Date().toISOString(),
                        points: pointValues
                    });

                    setData((prev) => {
                        const newData = [...prev];
                        let hasUpdates = false;

                        Object.entries(pointValues).forEach(([pvName, point]) => {
                          const pvIndex = newData.findIndex((d) => d.meta.name === pvName);
                          if (pvIndex >= 0) {
                              const value = typeof point.val === "number" 
                                  ? point.val 
                                  : Array.isArray(point.val) 
                                      ? point.val[0] 
                                      : NaN;

                              if (!isNaN(value)) {
                                  const timestamp = point.secs * 1000;
                                  
                                  addDebugLog("Processing live point", "debug", {
                                      pv: pvName,
                                      timestamp: new Date(timestamp).toISOString(),
                                      value: value
                                  });

                                  const newPoint = {
                                      timestamp,
                                      severity: point.severity || 0,
                                      status: point.status || 0,
                                      value,
                                      min: value,
                                      max: value,
                                      stddev: 0,
                                      count: 1,
                                  };

                                  // Add new point
                                  newData[pvIndex].data.push(newPoint);
                                  hasUpdates = true;

                                  // Update rolling window
                                  const now = Date.now();
                                  const oneHourAgo = now - 3600000;
                                  
                                  // Filter and sort
                                  newData[pvIndex].data = newData[pvIndex].data
                                      .filter(point => point.timestamp >= oneHourAgo)
                                      .sort((a, b) => a.timestamp - b.timestamp);

                                  addDebugLog("Updated dataset", "debug", {
                                      pv: pvName,
                                      pointCount: newData[pvIndex].data.length,
                                      timeRange: {
                                          start: new Date(oneHourAgo).toISOString(),
                                          end: new Date(now).toISOString()
                                      }
                                  });
                              }
                          }
                      });

                        if (hasUpdates) {
                            const now = new Date();
                            const duration = getTimeRangeDuration(timeRangeMode());
                            setTimeRange({
                                start: new Date(now.getTime() - duration),
                                end: now
                            });
                            setLastRefresh(now);
                        }

                        return hasUpdates ? newData : prev;
                    });
                },
                onError: (error) => {
                    addDebugLog("Live update error", "error", { error });
                    setError(error);
                    setRealTimeConfig((prev) => ({ ...prev, enabled: false }));
                }
            });
            
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            setError(errorMessage);
            addDebugLog("Failed to start live updates", "error", { error: errorMessage });
            setRealTimeConfig((prev) => ({ ...prev, enabled: false }));
            liveManager = undefined;
            return;
        }
    } else {
        try {
            addDebugLog("Stopping live mode", "info");
            
            if (liveManager) {
                await liveManager.stop();
                liveManager = undefined;
            }

            // Keep current time range for historical view
            const currentEnd = new Date();
            const duration = getTimeRangeDuration(timeRangeMode());
            setTimeRange({
                start: new Date(currentEnd.getTime() - duration),
                end: currentEnd
            });

            await handleRefresh();
            
            addDebugLog("Live updates stopped successfully", "info");
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            addDebugLog("Error stopping live updates", "error", { error: errorMessage });
            liveManager = undefined;
        }
        setRealTimeConfig((prev) => ({ ...prev, enabled: false }));
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
              onVisibilityChange={(pv, visible) => {
                setVisiblePVs((prev) => {
                  const newSet = new Set(prev);
                  if (visible) {
                    newSet.add(pv);
                  } else {
                    newSet.delete(pv);
                  }
                  return newSet;
                });
              }}
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

              {/* Action Buttons */}
              <div class="flex items-center gap-2">
                {/* Real-time Toggle */}
                <button
                  onClick={handleRealTimeToggle}
                  disabled={loading()}
                  class={`inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium 
                    ${
                      realTimeConfig().enabled
                        ? "bg-red-100 text-red-700 hover:bg-red-200"
                        : "bg-green-100 text-green-700 hover:bg-green-200"
                    } disabled:opacity-50 disabled:cursor-not-allowed transition-colors`}
                >
                  {realTimeConfig().enabled ? (
                    <>
                      <div class="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
                      <span>Live</span>
                    </>
                  ) : (
                    <span>Go Live</span>
                  )}
                </button>

                {/* Refresh Button */}
                <button
                  onClick={handleRefresh}
                  disabled={loading() || realTimeConfig().enabled}
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
                      pvs={selectedPVs().map((pv) => ({
                        name: pv.name,
                        pen: pv.pen,
                      }))}
                      timeRange={timeRange()}
                      timezone="UTC"
                    />
                  }
                >
                  <Dynamic
                    component={ChartJS}
                    data={visibleData()}
                    pvs={selectedPVs().map((pv) => ({
                      name: pv.name,
                      pen: pv.pen,
                    }))}
                    timeRange={timeRange()}
                    timezone="UTC"
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
              {realTimeConfig().enabled ? "Live Mode" : "Historical"}
            </span>
          </div>
          <TimeRangeSelector
            onChange={handleTimeRangeChange}
            disabled={loading() || realTimeConfig().enabled}
            initialTimezone={timezone()} // Changed from timezone to initialTimezone
          />
        </div>
      </div>

      {/* Debug Dialog */}
      <Show when={showDebugData()}>
      <DebugDialog
        isOpen={true}
        onClose={() => setShowDebugData(false)}
        data={debugLogs()}  // Ensure this passes the actual logs
      />
      </Show>

    </div>
  );
}
