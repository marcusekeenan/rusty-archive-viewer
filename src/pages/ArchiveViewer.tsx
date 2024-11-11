// src/components/ArchiveViewer.tsx
import { createSignal, createEffect, createMemo, onCleanup, onMount, Show } from "solid-js";
import { Controls } from "../components/controls/Controls";
import ChartJS from "../components/chart/ChartJS";
import {
  archiveStore,
  getPVMetadata,
  type NormalizedPVData,
  type PVWithProperties,
  type PenProperties,
  type YAxisConfig
} from "../utils/archiverApi";
import { PV_COLORS, DEFAULT_PEN_PROPERTIES } from "../types";
import { useChartConfig } from "../components/controls/common/hooks/useChartConfig";
import { useTimeRange } from "../components/controls/common/hooks/useTimeRange";
import { usePVManagement } from "../components/controls/common/hooks/usePVManagement";

const ArchiveViewer = () => {
  // Refs
  let chartContainerRef: HTMLDivElement | undefined;

  // Local State
  const [chartWidth, setChartWidth] = createSignal<number>(0);
  const [error, setError] = createSignal<string | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);

  // Use Custom Hooks
  const {
    selectedPVs,
    visiblePVs,
    handleAddPV,
    handleRemovePV,
    handleUpdatePV,
    handleVisibilityChange
  } = usePVManagement({
    initialPVs: archiveStore.state.pvConfigs,
    onError: (error) => setError(error.message)
  });

  const {
    timeRange,
    handleTimeRangeChange,
    timezone
  } = useTimeRange({
    initialRange: archiveStore.state.timeWindow,
    initialTimezone: archiveStore.state.chartConfig.timeZone
  });

  const {
    config: chartConfig,
    handleConfigChange,
    handleSaveConfig,
    handleLoadConfig
  } = useChartConfig({
    initialConfig: archiveStore.state.chartConfig
  });

  // Memoized Values
  const visibleData = createMemo(() => 
    (archiveStore.state.data || []).filter(data => 
      visiblePVs().has(data.meta.name)
    )
  );

  const totalPoints = createMemo(() => 
    visibleData().reduce((sum, pv) => sum + (pv.data?.length || 0), 0)
  );

  // Data Management
  const handleRefresh = async () => {
    if (archiveStore.state.isLive) return;

    try {
      setIsLoading(true);
      setError(null);

      await archiveStore.fetchData(
        Array.from(visiblePVs()),
        timeRange().start,
        timeRange().end,
        chartWidth(),
        timezone()
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch data');
      console.error("Error fetching data:", err);
    } finally {
      setIsLoading(false);
    }
  };

  // Live Mode Management
  const handleLiveToggle = async () => {
    try {
      if (archiveStore.state.isLive) {
        await archiveStore.stopLiveUpdates();
      } else {
        await archiveStore.startLiveUpdates();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to toggle live mode');
      console.error("Error toggling live mode:", err);
    }
  };

  // Effects
  createEffect(() => {
    const updateWidth = () => {
      if (chartContainerRef) {
        setChartWidth(chartContainerRef.clientWidth);
      }
    };

    window.addEventListener("resize", updateWidth);
    updateWidth();

    onCleanup(() => window.removeEventListener("resize", updateWidth));
  });

  createEffect(() => {
    if (!archiveStore.state.isLive && visiblePVs().size > 0) {
      handleRefresh();
    }
  });

  onCleanup(async () => {
    if (archiveStore.state.isLive) {
      await archiveStore.stopLiveUpdates();
    }
  });

  return (
    <div class="p-4 bg-gray-50 min-h-screen">
      <div class="grid grid-cols-[300px_1fr_300px] gap-4">
        {/* Left Panel - Controls */}
        <Controls 
          selectedPVs={selectedPVs}
          visiblePVs={visiblePVs}
          isLive={() => archiveStore.state.isLive}
          timeRange={timeRange}
          onAddPV={handleAddPV}
          onRemovePV={handleRemovePV}
          onTimeRangeChange={handleTimeRangeChange}
          onLiveModeToggle={handleLiveToggle}
          onSaveConfig={handleSaveConfig}
          onLoadConfig={handleLoadConfig}
        />

        {/* Center Panel - Chart */}
        <div class="space-y-4">
          {/* Status Bar */}
          <div class="bg-white rounded-lg shadow-sm p-4">
            <div class="flex justify-between items-center">
              {/* Live/Refresh Controls */}
              <div class="flex items-center gap-2">
                <button
                  onClick={handleLiveToggle}
                  disabled={isLoading() || visiblePVs().size === 0}
                  class={`
                    inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium
                    ${archiveStore.state.isLive 
                      ? "bg-red-100 text-red-700 hover:bg-red-200"
                      : "bg-green-100 text-green-700 hover:bg-green-200"
                    }
                    disabled:opacity-50 disabled:cursor-not-allowed transition-colors
                  `}
                >
                  {archiveStore.state.isLive ? (
                    <>
                      <div class="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
                      <span>Live</span>
                    </>
                  ) : (
                    <span>Go Live</span>
                  )}
                </button>

                <button
                  onClick={handleRefresh}
                  disabled={isLoading() || archiveStore.state.isLive || visiblePVs().size === 0}
                  class="
                    inline-flex items-center gap-2 px-3 py-1.5 bg-blue-50 text-blue-700
                    rounded-md text-sm font-medium hover:bg-blue-100
                    disabled:opacity-50 disabled:cursor-not-allowed transition-colors
                  "
                >
                  {isLoading() ? (
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

              {/* Error Display */}
              {error() && (
                <div class="text-red-600 text-sm">
                  Error: {error()}
                </div>
              )}
            </div>

            {/* Last Update Time */}
            {archiveStore.state.lastRefresh && (
              <div class="mt-2 text-sm text-gray-500">
                Last updated: {new Date(archiveStore.state.lastRefresh).toLocaleTimeString()}
              </div>
            )}
          </div>

          {/* Chart Area */}
          <div class="bg-white rounded-lg shadow-sm p-4">
            <div
              ref={chartContainerRef}
              class="w-full h-[calc(100vh-280px)] relative"
            >
              <Show
                when={visibleData().length > 0}
                fallback={
                  <div class="absolute inset-0 flex items-center justify-center text-gray-400">
                    No data to display
                  </div>
                }
              >
                <ChartJS
                  data={visibleData()}
                  pvs={selectedPVs()}
                  timeRange={timeRange()}
                  timezone={timezone()}
                />
              </Show>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ArchiveViewer;