// ArchiveViewer.tsx

import { createSignal, createEffect, onCleanup } from 'solid-js';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import PVSelector from '../components/controls/PVSelector';
import TimeRangeSelector from '../components/controls/TimeRangeSelector';
import EPICSChart from '../components/chart/EPICSChart';
import { fetchBinnedData } from '../utils/archiverApi';

// Constants and Configurations
const AUTO_REFRESH_INTERVAL = 30000; // 30 seconds
const MAX_RETRIES = 3;
const DEBUG_LOG_LIMIT = 50;

const getOperatorForTimeRange = (duration: number): string | null => {
  // Duration in milliseconds
  if (duration <= 15 * 60 * 1000) { // <= 15 minutes
    return null; // Raw data
  } else if (duration <= 2 * 60 * 60 * 1000) { // <= 2 hours
    return 'optimized_720'; // ~10s resolution
  } else if (duration <= 6 * 60 * 60 * 1000) { // <= 6 hours
    return 'optimized_720'; // ~30s resolution
  } else if (duration <= 24 * 60 * 60 * 1000) { // <= 24 hours
    return 'optimized_1440'; // ~1min resolution
  } else if (duration <= 7 * 24 * 60 * 60 * 1000) { // <= 7 days
    return 'optimized_2016'; // ~5min resolution
  } else {
    return 'optimized_4320'; // ~10min resolution
  }
};

// Debug Dialog Component
type DebugDialogProps = {
  isOpen: boolean;
  onClose: () => void;
  data: any;
};

const DebugDialog = (props: DebugDialogProps) => (
  <Dialog open={props.isOpen} onOpenChange={(isOpen) => {
    if (!isOpen) {
      props.onClose();
    }
  }}>
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

// Types for state management
type TimeRange = {
  start: Date;
  end: Date;
};

type DebugLog = {
  timestamp: string;
  message: string;
  type: 'info' | 'error' | 'debug' | 'success';
  details?: string | null;
};

// Main Component
const ArchiveViewer = () => {
  // Refs
  let chartContainer: HTMLDivElement | undefined;

  // State Management
  const [selectedPVs, setSelectedPVs] = createSignal<string[]>([]);
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
  const addDebugLog = (message: string, type: DebugLog['type'] = 'info', details: any = null) => {
    const log: DebugLog = {
      timestamp: new Date().toISOString(),
      message,
      type,
      details: details ? JSON.stringify(details, null, 2) : null,
    };
    setDebugLogs((prev) => [...prev.slice(-DEBUG_LOG_LIMIT + 1), log]);
    if (type === 'error') console.error(message, details);
    if (type === 'debug') console.debug(message, details);
  };

  // Data Fetching Logic
  const fetchData = async (retryCount = 0) => {
    try {
      if (!selectedPVs().length) {
        throw new Error('No PVs selected');
      }
      if (!timeRange().start || !timeRange().end) {
        throw new Error('Invalid time range');
      }

      const duration = timeRange().end.getTime() - timeRange().start.getTime();
      let operator: string | null = null;

      // Only use optimized for longer time ranges
      if (duration > 60 * 60 * 1000) {
        operator = currentOperator();
      }

      addDebugLog('Fetching data...', 'debug', {
        pvs: selectedPVs(),
        timeRange: {
          start: timeRange().start.toISOString(),
          end: timeRange().end.toISOString(),
          durationHours: duration / (1000 * 60 * 60),
        },
        operator,
        timezone: timezone(),
      });

      const responseData = await fetchBinnedData(
        selectedPVs(),
        timeRange().start,
        timeRange().end,
        {
          operator: operator || undefined,
          timezone: timezone(),
          chartWidth: chartContainer?.clientWidth || 1000,
        }
      );

      if (Array.isArray(responseData) && responseData.length > 0) {
        setData(responseData);
        setError(null);
        setLastRefresh(new Date());

        addDebugLog('Data fetch successful', 'success', {
          points: responseData[0]?.data?.length || 0,
        });
      } else {
        throw new Error('No data received');
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      addDebugLog(errorMessage, 'error', { error: err });
      // No retries - just show the error
    }
  };

  // Event Handlers
  const handleRefresh = async () => {
    setLoading(true);
    await fetchData();
    setLoading(false);
  };

  const handleTimeRangeChange = (start: Date, end: Date, operator: string | null) => {
    setTimeRange({ start, end });
    setCurrentOperator(operator);
    addDebugLog('Time range updated', 'info', {
      start: start.toISOString(),
      end: end.toISOString(),
      operator,
      duration: `${(end.getTime() - start.getTime()) / (1000 * 60 * 60)} hours`,
    });
  };

  // Auto-refresh Effect
  createEffect(() => {
    let interval: number | undefined;
    if (autoRefresh()) {
      interval = window.setInterval(handleRefresh, AUTO_REFRESH_INTERVAL);
      addDebugLog('Auto-refresh enabled', 'info', {
        interval: AUTO_REFRESH_INTERVAL / 1000 + ' seconds',
      });
    }
    onCleanup(() => {
      if (interval) {
        clearInterval(interval);
        addDebugLog('Auto-refresh disabled', 'info');
      }
    });
  });

  return (
    <div class="min-h-screen bg-gray-100">
      {/* Header */}
      <nav class="bg-blue-600 text-white p-4 shadow-lg">
        <div class="container mx-auto">
          <div class="flex justify-between items-center">
            <div>
              <h1 class="text-2xl font-bold">EPICS Archive Viewer</h1>
              <p class="text-sm mt-1 text-blue-100">
                Interactive data visualization for EPICS process variables
              </p>
            </div>
            {lastRefresh() && (
              <div class="text-sm text-blue-100">
                Last updated: {lastRefresh()?.toLocaleString()}
              </div>
            )}
          </div>
        </div>
      </nav>

      <main class="container mx-auto p-4 space-y-6">
        {/* Control Panels */}
        <div class="grid md:grid-cols-2 gap-6">
          {/* PV Selection Panel */}
          <div class="bg-white rounded-lg shadow-md p-6">
            <div class="flex justify-between items-center mb-4">
              <h2 class="text-lg font-semibold">Process Variables</h2>
              <span class="text-sm text-gray-500">
                {selectedPVs().length} PVs selected
              </span>
            </div>
            <PVSelector
              selectedPVs={selectedPVs}
              onAddPV={(pv) => {
                setSelectedPVs((prev) => [...prev, pv]);
                addDebugLog(`Added PV: ${pv}`, 'info');
              }}
              onRemovePV={(pv) => {
                setSelectedPVs((prev) => prev.filter((p) => p !== pv));
                addDebugLog(`Removed PV: ${pv}`, 'info');
              }}
            />
          </div>

          {/* Time Range Panel */}
          <div class="bg-white rounded-lg shadow-md p-6">
            <div class="flex justify-between items-center mb-4">
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
                addDebugLog(`Timezone changed to ${tz}`, 'info');
              }}
              disabled={loading()}
            />
          </div>
        </div>

        {/* Visualization Panel */}
        <div class="bg-white rounded-lg shadow-md p-6">
          <div class="flex flex-wrap justify-between items-center gap-4 mb-4">
            <h2 class="text-lg font-semibold">Data Visualization</h2>
            <div class="flex flex-wrap items-center gap-2">
              {/* Control Buttons */}
              <button
                onClick={() => setAutoRefresh(!autoRefresh())}
                class={`px-4 py-2 rounded text-white transition-colors ${
                  autoRefresh()
                    ? 'bg-red-500 hover:bg-red-600'
                    : 'bg-green-500 hover:bg-green-600'
                }`}
                disabled={loading()}
              >
                {autoRefresh() ? 'Stop Auto-refresh' : 'Start Auto-refresh'}
              </button>

              <button
                onClick={() => setShowDebugData(true)}
                class="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors"
                disabled={loading()}
              >
                View Raw Data
              </button>

              <button
                onClick={handleRefresh}
                disabled={loading()}
                class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 
                       disabled:opacity-50 disabled:cursor-not-allowed 
                       transition-colors flex items-center gap-2"
              >
                {loading() ? (
                  <>
                    <div class="animate-spin h-5 w-5 border-2 border-white border-t-transparent rounded-full" />
                    Fetching...
                  </>
                ) : (
                  <>
                    <span>Fetch Data</span>
                    {data().length > 0 && (
                      <span class="text-xs bg-blue-600 px-2 py-1 rounded">
                        {data().reduce((acc, pv) => acc + pv.data.length, 0)} points
                      </span>
                    )}
                  </>
                )}
              </button>
            </div>
          </div>

          {/* Error Display */}
          {error() && (
            <div class="mb-4 p-3 bg-red-100 text-red-700 rounded border border-red-200">
              <div class="font-semibold">Error</div>
              <div class="text-sm">{error()}</div>
            </div>
          )}

          {/* Chart */}
          <div ref={chartContainer} class="w-full min-h-[400px] relative">
            {data().length > 0 ? (
              <EPICSChart
                data={data()}
                pvs={selectedPVs()}
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

        {/* Debug Console */}
        <div class="bg-white rounded-lg shadow-md p-6">
          <div class="flex justify-between items-center mb-4">
            <div class="flex items-center gap-2">
              <h2 class="text-lg font-semibold">Debug Console</h2>
              <span class="text-xs bg-gray-200 px-2 py-1 rounded">
                {debugLogs().length} events
              </span>
            </div>
            <button
              onClick={() => setDebugLogs([])}
              class="text-sm text-gray-500 hover:text-gray-700"
            >
              Clear Logs
            </button>
          </div>
          <div class="h-48 overflow-y-auto font-mono text-sm bg-gray-50 p-3 rounded">
            {debugLogs().map((log) => (
              <div
                class={`mb-1 ${
                  log.type === 'error'
                    ? 'text-red-600'
                    : log.type === 'success'
                    ? 'text-green-600'
                    : 'text-gray-600'
                }`}
              >
                <div class="flex items-start gap-2">
                  <span class="text-gray-400">
                    {log.timestamp.split('T')[1].split('.')[0]}
                  </span>
                  <span>{log.message}</span>
                </div>
                {log.details && (
                  <div class="ml-14 text-xs text-gray-500 whitespace-pre-wrap">
                    {log.details}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Debug Data Dialog */}
        <DebugDialog
          isOpen={showDebugData()}
          onClose={() => setShowDebugData(false)}
          data={data()}
        />
      </main>
    </div>
  );
};

export default ArchiveViewer;
