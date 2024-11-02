// ArchiveViewer.tsx
import { createSignal, createEffect, onCleanup, Show, For } from "solid-js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import ChartJS from "../components/chart/ChartJS";
import ChartuPlot from "../components/chart/ChartuPlot"; // Change this import
import {
  fetchBinnedData,
  type ExtendedFetchOptions,
  type NormalizedPVData,
} from "../utils/archiverApi";
import type {
  PVWithProperties,
  PenProperties,
} from "../components/controls/types";

// Constants
const AUTO_REFRESH_INTERVAL = 30000; // 30 seconds
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
  const [autoRefresh, setAutoRefresh] = createSignal<boolean>(false);
  const [lastRefresh, setLastRefresh] = createSignal<Date | null>(null);

  type ChartType = "chartjs" | "uplot";
  const [selectedChart, setSelectedChart] = createSignal<ChartType>("chartjs");
  // Add this with your other state declarations
  const [processingMode, setProcessingMode] =
    createSignal<ProcessingMode>("mean");

  const totalPoints = () => {
    const allData = visibleData();
    return allData.reduce((sum, pv) => sum + (pv.data?.length || 0), 0);
  };

  // Computed value for visible data
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
        operator: processingMode(), // Add this line
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

  // Auto-refresh Effect
  createEffect(() => {
    let interval: number | undefined;
    if (autoRefresh()) {
      interval = window.setInterval(handleRefresh, AUTO_REFRESH_INTERVAL);
    }
    onCleanup(() => {
      if (interval) clearInterval(interval);
    });
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
              {/* Processing Mode Selector */}
              <div class="flex gap-4 items-center">
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

                <div class="flex gap-2 items-center ml-auto">
                  <button
                    onClick={() => setAutoRefresh(!autoRefresh())}
                    class={`px-4 py-1.5 rounded text-white transition-colors ${
                      autoRefresh()
                        ? "bg-red-500 hover:bg-red-600"
                        : "bg-green-500 hover:bg-green-600"
                    }`}
                    disabled={loading()}
                  >
                    {autoRefresh() ? "Stop Auto-refresh" : "Start Auto-refresh"}
                  </button>

                  <button
                    onClick={handleRefresh}
                    disabled={loading()}
                    title={`Total points: ${totalPoints().toLocaleString()}`} // Add tooltip
                    class="px-4 py-1.5 bg-blue-500 text-white rounded hover:bg-blue-600 
         disabled:opacity-50 disabled:cursor-not-allowed 
         transition-colors flex items-center justify-center gap-2"
                  >
                    {loading() ? (
                      <>
                        <div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                        Fetching...
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
                    <ChartuPlot
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
                  <ChartJS
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
            disabled={loading()}
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
