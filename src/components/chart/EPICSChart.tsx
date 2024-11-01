import { onMount, createEffect, createSignal, onCleanup, For } from 'solid-js';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import type { PVWithProperties } from '../../types';

type EPICSChartProps = {
  data: any[];
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  timezone: string;  // Required string without undefined
};

const EPICSChart = (props: EPICSChartProps) => {
  let chartRef: HTMLDivElement | undefined;
  let uPlotInstance: uPlot | null = null;
  const [isLoading, setIsLoading] = createSignal(false);
  const [processingMode, setProcessingMode] = createSignal('mean');

  const displayModes = [
    { value: 'raw', label: 'Raw Data' },
    { value: 'firstSample', label: 'First Sample' },
    { value: 'lastSample', label: 'Last Sample' },
    { value: 'firstFill', label: 'First Fill (with interpolation)' },
    { value: 'lastFill', label: 'Last Fill (with interpolation)' },
    { value: 'mean', label: 'Mean Value' },
    { value: 'min', label: 'Minimum Value' },
    { value: 'max', label: 'Maximum Value' },
    { value: 'count', label: 'Sample Count' },
    { value: 'median', label: 'Median Value' },
    { value: 'std', label: 'Standard Deviation' },
  ];

  // Helper function to get smart date formatting based on time range
  const getTimeFormat = (timeRange: number): (timestamp: number) => string => {
    const hours = timeRange / (3600 * 1000);
    
    if (hours <= 1) {
      return (ts: number) => new Date(ts).toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        timeZone: props.timezone
      });
    } else if (hours <= 24) {
      return (ts: number) => new Date(ts).toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        timeZone: props.timezone
      });
    } else if (hours <= 24 * 7) {
      return (ts: number) => new Date(ts).toLocaleString(undefined, {
        month: 'numeric',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
        timeZone: props.timezone
      });
    } else {
      return (ts: number) => new Date(ts).toLocaleDateString(undefined, {
        month: 'numeric',
        day: 'numeric',
        year: '2-digit',
        timeZone: props.timezone
      });
    }
  };

  const getProcessedValue = (point: any, mode: string) => {
    if (!point) return null;

    if (Array.isArray(point.val)) {
      const [mean, std, min, max, count] = point.val;
      switch (mode) {
        case 'mean': return mean;
        case 'min': return min;
        case 'max': return max;
        case 'std': return std;
        case 'count': return count;
        default: return mean;
      }
    } else {
      return point.value ?? point.val;
    }
  };

  const processData = (rawData: any[]) => {
    if (!rawData?.[0]?.data) return null;

    const processedData = rawData.map((pvData, index) => {
      const { data: points, meta, pen } = pvData;
      const mode = processingMode();

      const timestamps: number[] = [];
      const values: number[] = [];
      const mins: number[] = [];
      const maxs: number[] = [];
      let isStatisticalData = false;

      points.forEach((point: any) => {
        if (!point) return;

        const timestamp = point.timestamp || (point.secs * 1000);

        if (Array.isArray(point.val)) {
          isStatisticalData = true;
          const value = getProcessedValue(point, mode);

          if (typeof value === 'number' && !isNaN(value)) {
            timestamps.push(timestamp);
            values.push(value);

            if (mode === 'minmax' || mode === 'raw') {
              mins.push(point.val[2]);
              maxs.push(point.val[3]);
            } else {
              mins.push(value);
              maxs.push(value);
            }
          }
        } else {
          const value = point.value ?? point.val;
          if (typeof value === 'number' && !isNaN(value)) {
            timestamps.push(timestamp);
            values.push(value);
            mins.push(value);
            maxs.push(value);
          }
        }
      });

      return {
        name: meta?.name ?? 'Unknown',
        unit: meta?.EGU ?? '',
        data: [timestamps, values, mins, maxs],
        pen,
        isStatisticalData: isStatisticalData && (mode === 'minmax' || mode === 'raw'),
      };
    });

    // Get overall Y range
    const allValues = processedData.flatMap(pvData => 
      [...pvData.data[1], ...pvData.data[2], ...pvData.data[3]]
    );

    const minVal = Math.min(...allValues);
    const maxVal = Math.max(...allValues);
    const range = maxVal - minVal;
    const padding = range * 0.1;

    return {
      series: processedData,
      yRange: [minVal - padding, maxVal + padding]
    };
  };
  const initChart = () => {
    if (!chartRef || !props.data) return;

    const processedData = processData(props.data);
    if (!processedData?.series.length) return;

    if (uPlotInstance) {
      uPlotInstance.destroy();
      uPlotInstance = null;
    }

    chartRef.innerHTML = '';

    const parentWidth = chartRef.parentElement?.clientWidth || 800;
    const parentHeight = chartRef.parentElement?.clientHeight || 400;

    // Calculate time range for x-axis formatting
    const timeRange = processedData.series[0].data[0][processedData.series[0].data[0].length - 1] - 
                     processedData.series[0].data[0][0];
    const timeFormatter = getTimeFormat(timeRange);

    // Calculate number of x-axis ticks based on width
    const minTickWidth = 100; // Minimum pixels between ticks
    const maxTicks = Math.floor(parentWidth / minTickWidth);
    const tickSpacing = Math.ceil(timeRange / maxTicks);

    // Build series configuration
    const seriesConfig: uPlot.Series[] = [
      {
        label: "Time",
        value: (u: any, v: number | null) => timeFormatter(v || 0),
        stroke: "#000000" // Add the stroke property here
      }
    ];

    // Add series for each PV
    processedData.series.forEach((pvData) => {
      // Main series
      seriesConfig.push({
        label: `${pvData.name} (${pvData.unit})`,
        stroke: pvData.pen.color,
        width: pvData.pen.width,
        points: {
          show: pvData.pen.showPoints,
          size: pvData.pen.pointSize,
        },
        value: (u: any, v: number | null) => v?.toFixed(2) ?? 'N/A'
      });
    
      // Min/Max bands if statistical
      if (pvData.isStatisticalData) {
        seriesConfig.push(
          {
            label: `${pvData.name} Min`,
            stroke: pvData.pen.color,
            fill: pvData.pen.color,
            points: { show: false },
            value: (u: any, v: number | null) => v?.toFixed(2) ?? 'N/A'
          },
          {
            label: `${pvData.name} Max`,
            stroke: pvData.pen.color,
            points: { show: false },
            value: (u: any, v: number | null) => v?.toFixed(2) ?? 'N/A'
          }
        );
      }
    });

    const opts: uPlot.Options = {
      title: processedData.series.map(s => s.name).join(", "),
      width: parentWidth,
      height: parentHeight,
      series: seriesConfig as uPlot.Series[],
      scales: {
        x: {
          time: true,
          auto: true,
        },
        y: {
          auto: true,
        }
      },
      axes: [
        {
          scale: "x",
          side: 2,
          grid: { show: true, stroke: "#dedede" },
          ticks: { show: false },
          values: (u, vals) => vals.map(timeFormatter),
          space: 60,
          rotate: -20,
          gap: 15
        },
        {
          scale: "y",
          side: 3,
          grid: { show: true, stroke: "#dedede" },
          ticks: { show: false },
          values: (u, vals) => vals.map(v => v.toFixed(2)),
          size: 60
        }
      ],
      padding: [20, 50, 60, 60]
    };

    try {
      // Combine all data series
      const allData = [
        processedData.series[0].data[0], // timestamps
        ...processedData.series.flatMap(s => [
          s.data[1], // values
          ...(s.isStatisticalData ? [s.data[2], s.data[3]] : []) // min/max if statistical
        ])
      ];

      const typedData = allData.map(arr => new Float64Array(arr));
      uPlotInstance = new uPlot(opts, typedData, chartRef);
    } catch (error) {
      console.error('Error initializing chart:', error);
    }
  };

  onMount(() => {
    if (chartRef) {
      requestAnimationFrame(initChart);

      const resizeObserver = new ResizeObserver(() => {
        if (uPlotInstance && chartRef) {
          const width = chartRef.parentElement?.clientWidth || 800;
          const height = chartRef.parentElement?.clientHeight || 400;
          
          uPlotInstance.setSize({
            width,
            height,
          });
        }
      });

      resizeObserver.observe(chartRef.parentElement as Element);

      onCleanup(() => {
        resizeObserver.disconnect();
        if (uPlotInstance) {
          uPlotInstance.destroy();
        }
      });
    }
  });

  createEffect(() => {
    if (props.data && chartRef) {
      initChart();
    }
  });

  return (
    <div class="flex flex-col h-full">
      <div class="mb-4 flex-shrink-0">
        <label class="block mb-2 text-sm font-medium">Display Mode:</label>
        <select
          value={processingMode()}
          onChange={(e) => {
            setProcessingMode((e.target as HTMLSelectElement).value);
            if (props.data) {
              initChart();
            }
          }}
          class="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <For each={displayModes}>
            {(mode) => <option value={mode.value}>{mode.label}</option>}
          </For>
        </select>
      </div>

      <div class="relative flex-grow min-h-0">
        <div
          ref={chartRef}
          class="absolute inset-0"
        />

        {isLoading() && (
          <div class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-75">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
          </div>
        )}
      </div>

      <div class="mt-4 px-4 py-2 bg-gray-50 rounded text-sm flex-shrink-0">
        <div>
          {props.data?.[0]?.data ? (
            <For each={props.pvs}>
              {(pv) => (
                <div class="flex items-center gap-2">
                  <div 
                    class="w-3 h-3 rounded-full" 
                    style={{ "background-color": pv.pen.color }}
                  />
                  <span>{pv.name}</span>
                </div>
              )}
            </For>
          ) : (
            <div>No data available</div>
          )}
        </div>
      </div>
    </div>
  );
};

export default EPICSChart;