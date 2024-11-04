// ArchiveViewer.tsx
import {
  createSignal,
  createEffect,
  createMemo,
  onCleanup,
  Show,
  For,
} from "solid-js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import ChartJS from "../components/chart/ChartJS";
import ChartuPlot from "../components/chart/ChartuPlot";
import {
  fetchBinnedData,
  get_data_at_time,
  type ExtendedFetchOptions,
  type NormalizedPVData,
} from "../utils/archiverApi";
import type {
  PVWithProperties,
  PenProperties,
} from "../components/controls/types";

import { Dynamic } from "solid-js/web";

// Constants
const DEBUG_LOG_LIMIT = 50;

const DISPLAY_MODES = [
  { value: "raw", label: "Raw Data" },
  { value: "firstSample", label: "First Sample" },
  { value: "lastSample", label: "Last Sample" },
  { value: "firstFill", label: "First Fill (with interpolation)" },
  { value: "lastFill", label: "Last Fill (with interpolation)" },
  { value: "mean", label: "Mean Value" },
  { value: "min", label: "Minimum Value" },
  { value: "max", label: "Maximum Value" },
  { value: "count", label: "Sample Count" },
  { value: "median", label: "Median Value" },
  { value: "std", label: "Standard Deviation" },
] as const;

type ProcessingMode = (typeof DISPLAY_MODES)[number]["value"];

type TimeRange = {
  start: Date;
  end: Date;
};

type DebugLog = {
  timestamp: string;
  message: string;
  type: "info" | "error" | "debug" | "success";
  details?: string | null;
};

type DebugDialogProps = {
  isOpen: boolean;
  onClose: () => void;
  data: NormalizedPVData[];
};

type RealTimeMode = {
  enabled: boolean;
  updateInterval: number;
  lastTimestamp: number;
  bufferSize: number;
  operator: string;
};

// Debug Dialog Component
const DebugDialog = (props: DebugDialogProps) => (
  <Dialog
    open={props.isOpen}
    onOpenChange={(isOpen) => !isOpen && props.onClose()}
  >
    <DialogContent class="max-w-4xl max-h-[80vh]">
      <DialogHeader>
        <DialogTitle>Debug Information</DialogTitle>
      </DialogHeader>
      <div class="p-4 bg-gray-50 rounded">
        <div class="overflow-auto max-h-[60vh]">
          <pre class="whitespace-pre-wrap break-words">
            {JSON.stringify(props.data, null, 2)}
          </pre>
        </div>
      </div>
    </DialogContent>
  </Dialog>
);

export default function ArchiveViewer() {
  let chartContainer: HTMLDivElement | undefined;

  // State Management
  const [selectedPVs, setSelectedPVs] = createSignal<PVWithProperties[]>([]);
  const [visiblePVs, setVisiblePVs] = createSignal<Set<string>>(new Set());
  const [timeRange, setTimeRange] = createSignal<TimeRange>({
    start: new Date(Date.now() - 3600000), // Last hour by default
    end: new Date(),
  });
  const [currentOptions, setCurrentOptions] =
    createSignal<ExtendedFetchOptions>({});
  const [data, setData] = createSignal<NormalizedPVData[]>([]);
  const [loading, setLoading] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);
  const [debugLogs, setDebugLogs] = createSignal<DebugLog[]>([]);
  const [showDebugData, setShowDebugData] = createSignal<boolean>(false);
  const [lastRefresh, setLastRefresh] = createSignal<Date | null>(null);

  const [selectedChart, setSelectedChart] = createSignal<"chartjs" | "uplot">(
    "chartjs"
  );
  const [processingMode, setProcessingMode] =
    createSignal<ProcessingMode>("raw");

  const [realTimeMode, setRealTimeMode] = createSignal<RealTimeMode>({
    enabled: false,
    updateInterval: 1000,
    lastTimestamp: Date.now(),
    bufferSize: 3600, // 1 hour of second data
    operator: "raw",
  });

  // Computed values
  const chartKey = createMemo(() => {
    return `${data().length}-${lastRefresh()?.getTime()}`;
  });

  const totalPoints = () => {
    const allData = visibleData();
    return allData.reduce((sum, pv) => sum + (pv.data?.length || 0), 0);
  };

  const visibleData = () => {
    const allData = data();
    const visiblePVNames = visiblePVs();
    return allData.filter((pv) => visiblePVNames.has(pv.meta.name));
  };
  // Debug Logging
  const addDebugLog = (
    message: string,
    type: DebugLog["type"] = "info",
    details: any = null
  ) => {
    const log: DebugLog = {
      timestamp: new Date().toISOString(),
      message,
      type,
      details: details ? JSON.stringify(details, null, 2) : null,
    };
    setDebugLogs((prev) => [...prev.slice(-DEBUG_LOG_LIMIT + 1), log]);
    if (type === "error") console.error(message, details);
    if (type === "debug") console.debug(message, details);
  };

  // Utility functions
  const extractBinSize = (operator: string): number => {
    const match = operator.match(/_(\d+)$/);
    return match ? parseInt(match[1], 10) : 60; // default 1 minute
  };

  const mergeBinnedData = (
    oldData: NormalizedPVData[],
    newData: NormalizedPVData[],
    bufferSize: number
  ): NormalizedPVData[] => {
    return oldData.map((pvData) => {
      const newPVData = newData.find((d) => d.meta.name === pvData.meta.name);
      if (!newPVData) return pvData;

      return {
        ...pvData,
        data: [...pvData.data, ...newPVData.data]
          .sort((a, b) => a.timestamp - b.timestamp)
          .filter(
            (item, index, array) =>
              index === array.findIndex((t) => t.timestamp === item.timestamp)
          )
          .slice(-bufferSize),
      };
    });
  };

  // Handle PV visibility toggle
  const handlePVVisibilityToggle = (pvName: string, isVisible: boolean) => {
    setVisiblePVs((prev) => {
      const newSet = new Set(prev);
      if (isVisible) {
        newSet.add(pvName);
      } else {
        newSet.delete(pvName);
      }
      return newSet;
    });
    addDebugLog(`Toggled visibility for ${pvName}`, "debug", { isVisible });
  };

  // Real-time update logic
  const updateRealTimeData = async () => {
    if (!realTimeMode().enabled) return;
  
    try {
      const currentTime = new Date();
      const pvs = selectedPVs();
      
      if (processingMode() === 'raw') {
        // Convert timestamp to seconds since epoch
        const timestamp = Math.floor(currentTime.getTime() / 1000);
  
        addDebugLog('Fetching latest data', 'debug', {
          timestamp,
          pvCount: pvs.length,
          currentTime: currentTime.toISOString()
        });
  
        const latestData = await get_data_at_time(
          pvs.map(pv => pv.name),
          timestamp,  // Pass timestamp as seconds, not Date object
          { 
            fetch_latest_metadata: true,
            operator: processingMode()
          }
        );
  
        // Add more detailed logging
        addDebugLog('Received latest data', 'debug', {
          dataPoints: Object.keys(latestData).length,
          pvs: Object.keys(latestData),
          firstValue: Object.values(latestData)[0]
        });
  
        setData(prev => {
          const newData = [...prev];
          Object.entries(latestData).forEach(([pvName, point]) => {
            const pvIndex = newData.findIndex(d => d.meta.name === pvName);
            if (pvIndex >= 0) {
              const value = typeof point.val === 'number' ? point.val : 
                           Array.isArray(point.val) ? point.val[0] : 
                           NaN;
              
              if (!isNaN(value)) {
                newData[pvIndex].data.push({
                  timestamp: timestamp * 1000, // Convert back to milliseconds for display
                  severity: point.severity || 0,
                  status: point.status || 0,
                  value,
                  min: value,
                  max: value,
                  stddev: 0,
                  count: 1
                });
                // Maintain buffer size
                if (newData[pvIndex].data.length > realTimeMode().bufferSize) {
                  newData[pvIndex].data = newData[pvIndex].data.slice(-realTimeMode().bufferSize);
                }
              }
            }
          });
          return newData;
        });
  
        setRealTimeMode(prev => ({ ...prev, lastTimestamp: timestamp }));
        setLastRefresh(currentTime);
  
      } else {
        // Rest of the binned data logic remains the same
      }
  
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setError(errorMessage);
      addDebugLog('Real-time update failed', 'error', { 
        error: errorMessage,
        timestamp: new Date().toISOString()
      });
    }
  };

  // Data Fetching Logic
  const fetchData = async () => {
    try {
      const pvs = selectedPVs();
      if (pvs.length === 0) {
        throw new Error("No PVs selected");
      }

      const range = timeRange();
      if (!range.start || !range.end) {
        throw new Error("Invalid time range");
      }

      const options: ExtendedFetchOptions = {
        ...currentOptions(),
        chart_width: chartContainer?.clientWidth || 1000,
        operator: processingMode(),
      };

      const responseData = await fetchBinnedData(
        pvs.map((pv) => pv.name),
        range.start,
        range.end,
        options
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
          pointCount: dataWithProps.reduce(
            (sum, pv) => sum + pv.data.length,
            0
          ),
          timeRange: {
            start: range.start.toISOString(),
            end: range.end.toISOString(),
          },
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
    await fetchData();
    setLoading(false);
  };

  const handleTimeRangeChange = (
    start: Date,
    end: Date,
    options: ExtendedFetchOptions
  ) => {
    setTimeRange({ start, end });
    setCurrentOptions(options);
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

  const handleRealTimeToggle = () => {
    setRealTimeMode((prev) => {
      const newMode = {
        ...prev,
        enabled: !prev.enabled,
        lastTimestamp: Date.now(),
      };

      if (newMode.enabled) {
        // When enabling real-time, update time range end to now
        setTimeRange((prev) => ({
          ...prev,
          end: new Date(),
        }));
      }

      addDebugLog(
        `Real-time mode ${newMode.enabled ? "enabled" : "disabled"}`,
        "info",
        {
          operator: processingMode(),
          updateInterval: newMode.updateInterval,
        }
      );

      return newMode;
    });
  };

  // Effects
  createEffect(() => {
    let interval: number | undefined;

    if (realTimeMode().enabled) {
      // Initial update when enabling real-time
      updateRealTimeData();

      // Set up interval for subsequent updates
      interval = window.setInterval(() => {
        updateRealTimeData();

        // Also update the visible time range
        setTimeRange((prev) => ({
          ...prev,
          end: new Date(),
        }));
      }, realTimeMode().updateInterval);

      addDebugLog("Real-time updates started", "info", {
        interval: realTimeMode().updateInterval,
        operator: processingMode(),
      });
    }

    onCleanup(() => {
      if (interval) {
        clearInterval(interval);
        addDebugLog("Real-time updates stopped", "info");
      }
    });
  });

  // Effect to automatically adjust update interval based on processing mode
  createEffect(() => {
    const mode = processingMode();
    if (realTimeMode().enabled && mode !== "raw") {
      const binSize = extractBinSize(mode);
      setRealTimeMode((prev) => ({
        ...prev,
        updateInterval: Math.max(binSize * 1000, 1000), // Minimum 1 second interval
      }));

      addDebugLog("Adjusted real-time update interval", "debug", {
        mode,
        newInterval: Math.max(binSize * 1000, 1000),
      });
    }
  });

  // Effect to handle processing mode changes
  createEffect(() => {
    const mode = processingMode();
    addDebugLog("Processing mode changed", "debug", {
      mode,
      realTimeEnabled: realTimeMode().enabled,
    });

    if (realTimeMode().enabled) {
      // Force a refresh when changing modes in real-time
      handleRefresh();
    }
  });

  return (
    <div class="p-4">
      <div class="grid grid-cols-[350px_auto_350px] gap-4">
        {/* Left Side - PV Selector */}
        <div class="bg-white rounded-lg shadow-md p-4">
          <div class="flex justify-between items-center mb-2">
            <h2 class="text-lg font-semibold">Process Variables</h2>
            <span class="text-sm text-gray-500">
              {selectedPVs().length} PVs selected
            </span>
          </div>
          <PVSelector
            selectedPVs={selectedPVs}
            visiblePVs={visiblePVs}
            onAddPV={handleAddPV}
            onUpdatePV={handleUpdatePV}
            onRemovePV={handleRemovePV}
            onVisibilityChange={handlePVVisibilityToggle}
          />
        </div>

        {/* Middle Section - Controls and Chart */}
        <div class="space-y-4">
          {/* Control Buttons */}
          <div class="bg-white rounded-lg shadow-md p-4">
            <div class="flex flex-col gap-4">
              {/* Processing Mode and Controls Row */}
              <div class="flex gap-4 items-center">
                {/* Processing Mode Selector */}
                <div class="w-64">
                  <label class="block mb-2 text-sm font-medium text-gray-700">
                    Display Mode
                  </label>
                  <select
                    value={processingMode()}
                    onChange={(e) => {
                      setProcessingMode(
                        (e.target as HTMLSelectElement).value as ProcessingMode
                      );
                      handleRefresh();
                    }}
                    class="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                    disabled={loading()}
                  >
                    <For each={DISPLAY_MODES}>
                      {(mode) => (
                        <option value={mode.value}>{mode.label}</option>
                      )}
                    </For>
                  </select>
                </div>

                {/* Control Buttons */}
                <div class="flex gap-2 items-center ml-auto">
                  {/* Real-time Toggle Button */}
                  <button
                    onClick={handleRealTimeToggle}
                    disabled={loading()}
                    class={`px-4 py-1.5 rounded text-white transition-colors ${
                      realTimeMode().enabled
                        ? "bg-red-500 hover:bg-red-600"
                        : "bg-green-500 hover:bg-green-600"
                    } disabled:opacity-50 disabled:cursor-not-allowed`}
                  >
                    <div class="flex items-center gap-2">
                      {realTimeMode().enabled ? (
                        <>
                          <div class="w-2 h-2 rounded-full bg-white animate-pulse" />
                          <span>Live</span>
                        </>
                      ) : (
                        <span>Go Live</span>
                      )}
                    </div>
                  </button>

                  {/* Manual Refresh Button */}
                  <button
                    onClick={handleRefresh}
                    disabled={loading() || realTimeMode().enabled}
                    title={`Total points: ${totalPoints().toLocaleString()}`}
                    class="px-4 py-1.5 bg-blue-500 text-white rounded hover:bg-blue-600 
                           disabled:opacity-50 disabled:cursor-not-allowed 
                           transition-colors flex items-center justify-center gap-2"
                  >
                    {loading() ? (
                      <>
                        <div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                        <span>Fetching...</span>
                      </>
                    ) : (
                      <div class="flex items-center gap-2">
                        <span>Fetch Data</span>
                        <span class="text-xs bg-blue-600 px-2 py-0.5 rounded">
                          {totalPoints().toLocaleString()} pts
                        </span>
                      </div>
                    )}
                  </button>

                  <button
                    onClick={() => setShowDebugData(true)}
                    class="px-4 py-1.5 bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors"
                    disabled={loading()}
                  >
                    View Raw Data
                  </button>
                </div>
              </div>

              {/* Last Update Indicator */}
              {lastRefresh() && (
                <div class="text-xs text-gray-500">
                  Last updated: {lastRefresh()?.toLocaleTimeString()}
                </div>
              )}
            </div>
          </div>

          {/* Chart */}
          <div class="bg-white rounded-lg shadow-md p-4">
            {/* Chart Type Selector */}
            <div class="flex items-center gap-2 mb-4 border-b">
              <button
                class={`px-4 py-2 text-sm font-medium transition-colors relative ${
                  selectedChart() === "chartjs"
                    ? "text-blue-600 border-b-2 border-blue-600 -mb-px"
                    : "text-gray-500 hover:text-gray-700"
                }`}
                onClick={() => setSelectedChart("chartjs")}
              >
                Chart.js
              </button>
              <button
                class={`px-4 py-2 text-sm font-medium transition-colors relative ${
                  selectedChart() === "uplot"
                    ? "text-blue-600 border-b-2 border-blue-600 -mb-px"
                    : "text-gray-500 hover:text-gray-700"
                }`}
                onClick={() => setSelectedChart("uplot")}
              >
                µPlot
              </button>
            </div>

            {/* Error Display */}
            {error() && (
              <div class="mb-4 p-4 bg-red-100 text-red-700 rounded border border-red-200">
                <div class="font-semibold">Error</div>
                <div class="text-sm">{error()}</div>
              </div>
            )}

            {/* Chart Container */}
            <div
              ref={chartContainer}
              class="w-full mx-auto h-[calc(100vh-290px)] relative overflow-hidden"
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
                      timezone={currentOptions().timezone || "UTC"}
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
                    timezone={currentOptions().timezone || "UTC"}
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

        {/* Right Side - Time Range */}
        <div class="bg-white rounded-lg shadow-md p-4">
          <div class="flex justify-between items-center mb-2">
            <h2 class="text-lg font-semibold">Time Range</h2>
            {currentOptions().operator && (
              <span class="text-sm text-gray-500">
                Using {currentOptions().operator} operator
              </span>
            )}
          </div>
          <TimeRangeSelector
            onChange={handleTimeRangeChange}
            disabled={loading() || realTimeMode().enabled}
          />
        </div>
      </div>

      {showDebugData() && (
        <DebugDialog
          isOpen={showDebugData()}
          onClose={() => setShowDebugData(false)}
          data={visibleData()}
        />
      )}
    </div>
  );
}
