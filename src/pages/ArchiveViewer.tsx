import { createSignal, createEffect, onCleanup } from "solid-js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import PVSelector from "../components/controls/PVSelector";
import TimeRangeSelector from "../components/controls/TimeRangeSelector";
import EPICSChart from "../components/chart/EPICSChart";
import { fetchBinnedData } from "../utils/archiverApi";
import type { PVWithProperties, PenProperties } from "../types";
import { DEFAULT_PEN_PROPERTIES } from "../types";

// Constants
const AUTO_REFRESH_INTERVAL = 30000; // 30 seconds
const DEBUG_LOG_LIMIT = 50;

// Types
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
  data: any;
};

// Helper functions
const getOperatorForTimeRange = (duration: number): string | null => {
  if (duration <= 15 * 60 * 1000) { // <= 15 minutes
    return null; // Raw data
  } else if (duration <= 2 * 60 * 60 * 1000) { // <= 2 hours
    return "optimized_720"; // ~10s resolution
  } else if (duration <= 6 * 60 * 60 * 1000) { // <= 6 hours
    return "optimized_720"; // ~30s resolution
  } else if (duration <= 24 * 60 * 60 * 1000) { // <= 24 hours
    return "optimized_1440"; // ~1min resolution
  } else if (duration <= 7 * 24 * 60 * 60 * 1000) { // <= 7 days
    return "optimized_2016"; // ~5min resolution
  } else {
    return "optimized_4320"; // ~10min resolution
  }
};

// Debug Dialog Component
const DebugDialog = (props: DebugDialogProps) => (
  <Dialog
    open={props.isOpen}
    onOpenChange={(isOpen) => {
      if (!isOpen) {
        props.onClose();
      }
    }}
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

// Main Component
const ArchiveViewer = () => {
  // Refs
  let chartContainer: HTMLDivElement | undefined;

  // State Management
  const [selectedPVs, setSelectedPVs] = createSignal<PVWithProperties[]>([]);
  const [timeRange, setTimeRange] = createSignal<TimeRange>({
    start: new Date(),
    end: new Date(),
  });
  const [timezone, setTimezone] = createSignal<string>(
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  const [currentOperator, setCurrentOperator] = createSignal<string | null>(null);
  const [data, setData] = createSignal<any[]>([]);
  const [loading, setLoading] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);
  const [debugLogs, setDebugLogs] = createSignal<DebugLog[]>([]);
  const [showDebugData, setShowDebugData] = createSignal<boolean>(false);
  const [autoRefresh, setAutoRefresh] = createSignal<boolean>(false);
  const [lastRefresh, setLastRefresh] = createSignal<Date | null>(null);

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

  // Data Fetching Logic
  const fetchData = async () => {
    try {
      const pvs = selectedPVs();
      if (pvs.length === 0) {
        throw new Error("No PVs selected");
      }
      if (!timeRange().start || !timeRange().end) {
        throw new Error("Invalid time range");
      }

      const duration = timeRange().end.getTime() - timeRange().start.getTime();
      let operator: string | null = currentOperator();

      // Only use optimized for longer time ranges
      if (duration > 60 * 60 * 1000) {
        operator = getOperatorForTimeRange(duration);
      }

      addDebugLog("Fetching data...", "debug", {
        pvs: pvs.map(pv => pv.name),
        timeRange: {
          start: timeRange().start.toISOString(),
          end: timeRange().end.toISOString(),
          durationHours: duration / (1000 * 60 * 60),
        },
        operator,
        timezone: timezone(),
      });

      const responseData = await fetchBinnedData(
        pvs.map(pv => pv.name),
        timeRange().start,
        timeRange().end,
        {
          operator: operator || undefined,
          timezone: timezone(),
          chartWidth: chartContainer?.clientWidth || 1000,
        }
      );

      if (Array.isArray(responseData) && responseData.length > 0) {
        const dataWithProps = responseData.map((data, index) => ({
          ...data,
          pen: pvs[index].pen
        }));
        setData(dataWithProps);
        setError(null);
        setLastRefresh(new Date());

        addDebugLog("Data fetch successful", "success", {
          points: responseData[0]?.data?.length || 0,
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
    operator: string | null
  ) => {
    setTimeRange({ start, end });
    setCurrentOperator(operator);
    handleRefresh();
  };

  const handleAddPV = (pv: string, properties: PenProperties) => {
    setSelectedPVs(prev => [...prev, { name: pv, pen: properties }]);
    addDebugLog(`Added PV: ${pv}`, "info");
    handleRefresh();
  };

  const handleUpdatePV = (pv: string, properties: PenProperties) => {
    setSelectedPVs(prev => prev.map(p => 
      p.name === pv ? { ...p, pen: properties } : p
    ));
    addDebugLog(`Updated PV properties: ${pv}`, "info");
    handleRefresh();
  };

  const handleRemovePV = (pv: string) => {
    setSelectedPVs(prev => prev.filter(p => p.name !== pv));
    addDebugLog(`Removed PV: ${pv}`, "info");
    handleRefresh();
  };

  // Auto-refresh Effect
  createEffect(() => {
    let interval: number | undefined;
    if (autoRefresh()) {
      interval = window.setInterval(handleRefresh, AUTO_REFRESH_INTERVAL);
      addDebugLog("Auto-refresh enabled", "info", {
        interval: AUTO_REFRESH_INTERVAL / 1000 + " seconds",
      });
    }
    onCleanup(() => {
      if (interval) {
        clearInterval(interval);
        addDebugLog("Auto-refresh disabled", "info");
      }
    });
  });

  return (
    <div class="p-4">
      <div class="grid grid-cols-[320px_auto_320px] gap-4">
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
            onAddPV={handleAddPV}
            onUpdatePV={handleUpdatePV}
            onRemovePV={handleRemovePV}
          />
        </div>

        {/* Middle Section - Controls and Chart */}
        <div class="space-y-4">
          {/* Control Buttons */}
          <div class="bg-white rounded-lg shadow-md p-2">
            <div class="flex justify-end gap-2">
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
                  <span>Fetch Data</span>
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

          {/* Chart */}
          <div class="bg-white rounded-lg shadow-md p-4">
            {error() && (
              <div class="mb-4 p-3 bg-red-100 text-red-700 rounded border border-red-200">
                <div class="font-semibold">Error</div>
                <div class="text-sm">{error()}</div>
              </div>
            )}

            <div
              ref={chartContainer}
              class="w-full max-w-screen-xl mx-auto h-[calc(100vh-240px)] relative overflow-hidden"
            >
              {data().length > 0 ? (
                <EPICSChart
                  data={data()}
                  pvs={selectedPVs().map(pv => ({ name: pv.name, pen: pv.pen }))}
                  timeRange={timeRange()}
                  timezone={timezone()}
                />
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
            {currentOperator() && (
              <span class="text-sm text-gray-500">
                Using {currentOperator()} operator
              </span>
            )}
          </div>
          <TimeRangeSelector
            onChange={handleTimeRangeChange}
            onTimezoneChange={(tz) => {
              setTimezone(tz);
              addDebugLog(`Timezone changed to ${tz}`, "info");
            }}
            disabled={loading()}
          />
        </div>
      </div>

      {showDebugData() && (
        <DebugDialog
          isOpen={showDebugData()}
          onClose={() => setShowDebugData(false)}
          data={data()}
        />
      )}
    </div>
  );
};

export default ArchiveViewer;