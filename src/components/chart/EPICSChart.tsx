import { onMount, createEffect, onCleanup, createMemo } from "solid-js";
import { Chart as ChartJS, CategoryScale, LinearScale, PointElement, LineElement, 
         TimeScale, Title, Tooltip, Legend, ChartOptions, ChartDataset, LineController } from "chart.js";
import zoomPlugin from "chartjs-plugin-zoom";
import "chartjs-adapter-date-fns";
import type { PVWithProperties, AxisConfig, Meta, EPICSData } from "../../types";

ChartJS.register(LineController, CategoryScale, LinearScale, PointElement, LineElement, 
                TimeScale, Title, Tooltip, Legend, zoomPlugin);

interface ChartProps {
  data: EPICSData;
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  timezone: string;
  axes: Map<string, AxisConfig>;
  onAxisRangeUpdate?: (axisId: string, range: { low: number; high: number }) => void;
}

type Point = { x: number; y: number };
type Dataset = ChartDataset<"line", Point[]>;

export default function EPICSChart(props: ChartProps) {
  let chartRef: HTMLCanvasElement | undefined;
  let chart: ChartJS | null = null;

  const getAxisLimits = (axis: AxisConfig) => ({
    min: axis.autoRange ? undefined : axis.range?.low,
    max: axis.autoRange ? undefined : axis.range?.high
  });

  const formatDatasets = createMemo(() => {
    // Map each series to a dataset
    return props.data.series.map((seriesData, index) => {
      const pvInfo = props.pvs[index]; // Assume series index matches PV index
      if (!pvInfo) return null;

      const points: Point[] = props.data.timestamps.map((ts, i) => ({
        x: ts,
        y: seriesData[i]
      }));

      return {
        type: 'line' as const,
        label: `${props.data.meta[index]?.name || pvInfo.name} (${props.data.meta[index]?.EGU || ''})`,
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
      } as Dataset;
    }).filter((d): d is Dataset => d !== null);
  });

  createEffect(() => {
    const datasets = formatDatasets();
    if (!datasets.length || !chartRef) return;

    const config = {
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
            min: props.timeRange.start.getTime(),
            max: props.timeRange.end.getTime()
          },
          ...Object.fromEntries(
            Array.from(props.axes.entries()).map(([id, axis]) => [
              id,
              {
                type: 'linear',
                position: axis.position || 'left',
                title: {
                  display: true,
                  text: axis.EGU,
                  font: { size: 12 }
                },
                ...getAxisLimits(axis),
                afterDataLimits: (scale) => {
                  if (axis.autoRange && props.onAxisRangeUpdate) {
                    props.onAxisRangeUpdate(id, {
                      low: scale.min,
                      high: scale.max
                    });
                  }
                },
                grid: {
                  color: 'rgba(0,0,0,0.05)',
                  drawOnChartArea: axis.position === 'left'
                }
              }
            ])
          )
        },
        plugins: {
          tooltip: {
            mode: 'nearest',
            intersect: false,
            callbacks: {
              title: ctx => new Date(ctx[0].parsed.x).toLocaleString(undefined, { 
                timeZone: props.timezone 
              }),
              label: ctx => {
                const axis = props.axes.get(ctx.dataset.yAxisID || '');
                return `${ctx.dataset.label}: ${ctx.parsed.y.toFixed(2)} ${axis?.EGU || ''}`;
              }
            }
          },
          zoom: {
            limits: {
              x: {
                min: props.timeRange.start.getTime(),
                max: props.timeRange.end.getTime()
              }
            },
            pan: { 
              enabled: true, 
              mode: 'x', 
              modifierKey: 'shift' 
            },
            zoom: {
              wheel: { 
                enabled: true, 
                modifierKey: 'ctrl' 
              },
              pinch: { 
                enabled: true 
              },
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
    
    if (chart) {
      chart.data = config.data;
      chart.options = config.options;
      chart.update('none');
    } else {
      chart = new ChartJS(chartRef, config);
    }
  });

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => chart?.resize());
    if (chartRef?.parentElement) resizeObserver.observe(chartRef.parentElement);
    onCleanup(() => {
      resizeObserver.disconnect();
      chart?.destroy();
    });
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
            class="ml-auto px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 text-xs"
            onClick={() => chart?.resetZoom()}
          >
            Reset Zoom
          </button>
        </div>
      </div>
    </div>
  );
}