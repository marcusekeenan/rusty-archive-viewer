import { onMount, createEffect, onCleanup, createMemo } from "solid-js";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  TimeScale,
  Title,
  Tooltip,
  Legend,
  ChartOptions,
  ChartDataset,
  LineController
} from "chart.js";
import zoomPlugin from "chartjs-plugin-zoom";
import "chartjs-adapter-date-fns";
import type { PVWithProperties, AxisConfig, UPlotData } from "../../types";

ChartJS.register(
  LineController,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  TimeScale,
  Title,
  Tooltip,
  Legend,
  zoomPlugin
);

interface ChartProps {
  data: UPlotData;
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  axes: Map<string, AxisConfig>;
}

type Dataset = ChartDataset<"line", { x: number; y: number }[]>;

export default function EPICSChart(props: ChartProps) {
  let chartRef: HTMLCanvasElement | undefined;
  let chart: ChartJS | null = null;

  const formatDatasets = createMemo(() => {
    const datasets: Dataset[] = [];
    
    props.data.series.forEach((series, index) => {
      const meta = props.data.meta[index];
      const pvInfo = props.pvs.find(pv => pv.name === meta.name);
      if (!pvInfo) return;

      // Convert parallel arrays to point objects
      const points = series.map((value, i) => ({
        x: props.data.timestamps[i],
        y: value
      })).filter(p => !isNaN(p.y));

      if (!points.length) return;

      datasets.push({
        type: 'line',
        label: `${meta.name} (${meta.EGU || ''})`,
        data: points,
        yAxisID: pvInfo.axisId || 'default',
        borderColor: pvInfo.pen.color,
        backgroundColor: pvInfo.pen.color,
        borderWidth: pvInfo.pen.lineWidth,
        borderDash: pvInfo.pen.style === 'dashed' ? [8,4] : 
                   pvInfo.pen.style === 'dotted' ? [2,2] : [],
        pointRadius: pvInfo.pen.showPoints ? pvInfo.pen.pointSize : 0,
        pointHoverRadius: pvInfo.pen.showPoints ? pvInfo.pen.pointSize + 2 : 4,
        tension: 0.1,
        fill: false
      });
    });

    return datasets;
  });

  const createChartConfig = (datasets: Dataset[]) => {
    const yAxes = Object.fromEntries(
      Array.from(props.axes.entries()).map(([id, axis]) => [
        id,
        {
          type: 'linear' as const,
          position: axis.position || 'left',
          title: {
            display: true,
            text: axis.EGU,
            font: { size: 12 },
            color: '#666'
          },
          grid: {
            color: 'rgba(0,0,0,0.05)',
            drawTicks: false
          },
          ticks: {
            color: '#666',
            font: { size: 11 }
          },
          min: axis.autoRange ? undefined : axis.range?.low,
          max: axis.autoRange ? undefined : axis.range?.high
        }
      ])
    );

    return {
      type: 'line' as const,
      data: { datasets },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        parsing: false,
        normalized: true,
        layout: {
          padding: { top: 30, right: 20, bottom: 45, left: 20 }
        },
        interaction: {
          mode: 'nearest',
          axis: 'x',
          intersect: false
        },
        scales: {
          x: {
            type: 'time',
            time: {
              displayFormats: {
                millisecond: 'HH:mm:ss.SSS',
                second: 'HH:mm:ss',
                minute: 'HH:mm',
                hour: 'HH:mm',
                day: 'MM/dd'
              },
              tooltipFormat: 'yyyy-MM-dd HH:mm:ss.SSS'
            },
            grid: { color: 'rgba(0,0,0,0.05)' },
            ticks: {
              maxRotation: 0,
              autoSkip: true,
              maxTicksLimit: 8,
              font: { size: 11 }
            }
          },
          ...yAxes
        },
        plugins: {
          legend: {
            position: 'top' as const,
            align: 'start' as const,
            labels: {
              usePointStyle: true,
              pointStyle: 'circle',
              padding: 15,
              boxWidth: 8,
              boxHeight: 8,
              color: '#333',
              font: { size: 11 }
            }
          },
          tooltip: {
            mode: 'index',
            intersect: false,
            backgroundColor: 'rgba(255,255,255,0.95)',
            titleColor: '#333',
            bodyColor: '#333',
            borderColor: '#ddd',
            borderWidth: 1,
            padding: 10,
            callbacks: {
              title: ctx => {
                if (!ctx[0]?.parsed?.x) return '';
                return new Date(ctx[0].parsed.x).toLocaleString();
              },
              label: ctx => {
                if (!ctx.dataset.yAxisID) return '';
                const axis = props.axes.get(ctx.dataset.yAxisID);
                return `${ctx.dataset.label}: ${ctx.parsed.y.toFixed(3)} ${axis?.EGU || ''}`;
              }
            }
          },
          zoom: {
            pan: { enabled: true, mode: 'x', modifierKey: 'shift' },
            zoom: {
              wheel: { enabled: true, modifierKey: 'ctrl' },
              pinch: { enabled: true },
              mode: 'x',
              drag: {
                enabled: true,
                backgroundColor: 'rgba(127,127,127,0.2)'
              }
            }
          }
        }
      } as ChartOptions<'line'>
    };
  };

  createEffect(() => {
    const datasets = formatDatasets();
    if (!datasets.length || !chartRef) return;

    const config = createChartConfig(datasets);
    
    if (chart) {
      chart.data = config.data;
      chart.options = config.options;
      chart.update('none');
    } else {
      chart = new ChartJS(chartRef, config);
    }
  });

  onMount(() => {
    if (chartRef?.parentElement) {
      const resizeObserver = new ResizeObserver(() => chart?.resize());
      resizeObserver.observe(chartRef.parentElement);
      onCleanup(() => {
        resizeObserver.disconnect();
        chart?.destroy();
        chart = null;
      });
    }
  });

  return (
    <div class="relative w-full h-full overflow-hidden bg-white">
      <canvas ref={chartRef} class="w-full h-full" />
      <div class="absolute bottom-1 left-0 right-0 bg-white/80 text-xs py-1 px-2 border-t">
        <div class="flex items-center gap-4">
          <div class="text-gray-600">⇧ + Drag to Pan</div>
          <div class="text-gray-600">Ctrl + Wheel to Zoom</div>
          <div class="text-gray-600">Drag to Box Zoom</div>
          <button
            class="ml-auto px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 text-xs transition-colors"
            onClick={() => chart?.resetZoom()}
          >
            Reset Zoom
          </button>
        </div>
      </div>
    </div>
  );
}