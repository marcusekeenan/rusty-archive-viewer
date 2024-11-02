// EPICSChart.tsx
import { onMount, createEffect, onCleanup } from 'solid-js';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import type { PVWithProperties } from '../controls/types';

type EPICSChartProps = {
  data: any[];
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  timezone: string;
};

export default function ChartuPlot(props: EPICSChartProps) {
  let chartRef: HTMLDivElement | undefined;
  let uPlotInstance: uPlot | null = null;

  // Helper function to format timestamps based on range
  const getTimeFormat = (timeRange: number): (timestamp: number) => string => {
    const hours = timeRange / (3600 * 1000);
    
    if (hours <= 1) {
      return (ts: number) => new Date(ts).toLocaleTimeString(undefined, {
        hour: '2-digit', minute: '2-digit', second: '2-digit',
        timeZone: props.timezone
      });
    } else if (hours <= 24) {
      return (ts: number) => new Date(ts).toLocaleTimeString(undefined, {
        hour: '2-digit', minute: '2-digit',
        timeZone: props.timezone
      });
    } else if (hours <= 24 * 7) {
      return (ts: number) => new Date(ts).toLocaleString(undefined, {
        month: 'numeric', day: 'numeric',
        hour: '2-digit', minute: '2-digit',
        timeZone: props.timezone
      });
    } else {
      return (ts: number) => new Date(ts).toLocaleDateString(undefined, {
        month: 'numeric', day: 'numeric', year: '2-digit',
        timeZone: props.timezone
      });
    }
  };

  function createTooltipPlugin() {
    let tooltip: HTMLDivElement;
    let over: HTMLElement;
  
    function syncBounds() {
      const rect = over.getBoundingClientRect();
      tooltip.style.left = `${rect.left}px`;
      tooltip.style.top = `${rect.top}px`;
      tooltip.style.width = `${rect.width}px`;
      tooltip.style.height = `${rect.height}px`;
    }
  
    const tooltipPlugin = {
      hooks: {
        init: (u: uPlot) => {
          // Create tooltip container that overlays the chart
          tooltip = document.createElement('div');
          tooltip.className = 'fixed pointer-events-none';
          
          // Create tooltip content
          const tooltipContent = document.createElement('div');
          tooltipContent.className = 'absolute z-50 bg-white/90 shadow-lg border rounded px-3 py-2 text-sm hidden';
          tooltip.appendChild(tooltipContent);
          
          document.body.appendChild(tooltip);
          over = u.over;
          
          // Initial position sync
          syncBounds();
  
          over.addEventListener('mouseleave', () => {
            tooltipContent.classList.add('hidden');
          });
  
          over.addEventListener('mouseenter', () => {
            tooltipContent.classList.remove('hidden');
          });
        },
        setSize: () => {
          syncBounds();
        },
        setCursor: (u: uPlot) => {
          const { left, top, idx } = u.cursor;
          const tooltipContent = tooltip.children[0] as HTMLDivElement;
          
          if (left === null || top === null || idx === null) {
            tooltipContent.classList.add('hidden');
            return;
          }
          
          const timestamp = u.posToVal(left ?? 0, 'x');
          let html = `<div class="font-medium mb-1">
            ${new Date(timestamp).toLocaleString(undefined, {
              timeZone: props.timezone,
              dateStyle: 'short',
              timeStyle: 'medium'
            })}
          </div>`;
  
          html += '<div class="space-y-1">';
          for (let i = 1; i < u.series.length; i++) {
            const series = u.series[i];
            if (!series.show) continue;
  
            const value = idx !== null && idx !== undefined ? u.data[i][idx] : null;
            if (value != null) {
              html += `
                <div class="flex items-center gap-2">
                  <div class="w-2 h-2 rounded-full" style="background-color: ${series.stroke}"></div>
                  <span>${series.label}: ${value.toFixed(2)}</span>
                </div>
              `;
            }
          }
          html += '</div>';
  
          tooltipContent.innerHTML = html;
          tooltipContent.classList.remove('hidden');
  
          // Position tooltip content relative to cursor
          const rect = over.getBoundingClientRect();
          const tooltipRect = tooltipContent.getBoundingClientRect();
          
          let tLeft = left !== undefined ? left : 0;
          let tTop = top ?? 0;
  
          // Adjust position to keep tooltip within chart bounds
          if ((left ?? 0) + tooltipRect.width > rect.width) {
            tLeft = (left ?? 0) - tooltipRect.width - 10;
          } else {
            tLeft += 10;
          }
  
          if ((top ?? 0) + tooltipRect.height > rect.height) {
            tTop = (top ?? 0) - tooltipRect.height - 10;
          } else {
            tTop += 10;
          }
  
          tooltipContent.style.transform = `translate(${tLeft}px, ${tTop}px)`;
        },
        destroy: () => {
          tooltip?.remove();
        }
      }
    };
  
    return tooltipPlugin;
  }

  const processData = (rawData: any[]) => {
    if (!rawData?.[0]?.data) return null;

    const processedData = rawData.map((pvData) => {
      const { data: points, meta, pen } = pvData;
      const timestamps: number[] = [];
      const values: number[] = [];

      points.forEach((point: any) => {
        if (!point) return;
        const timestamp = point.timestamp || (point.secs * 1000);
        const value = point.value ?? point.val;
        
        if (typeof value === 'number' && !isNaN(value)) {
          timestamps.push(timestamp);
          values.push(value);
        }
      });

      return {
        name: meta?.name ?? 'Unknown',
        unit: meta?.EGU ?? '',
        data: [timestamps, values],
        pen
      };
    });

    // Calculate Y axis range with padding
    const allValues = processedData.flatMap(pvData => pvData.data[1]);
    const minVal = Math.min(...allValues);
    const maxVal = Math.max(...allValues);
    const range = maxVal - minVal;
    const padding = range * 0.1;

    return {
      series: processedData,
      yRange: [minVal - padding, maxVal + padding] as const
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

    const timeRange = processedData.series[0].data[0][processedData.series[0].data[0].length - 1] - 
                     processedData.series[0].data[0][0];
    const timeFormatter = getTimeFormat(timeRange);

    const seriesConfig: uPlot.Series[] = [
      {
        label: "Time",
        value: (u: any, v: number | null) => timeFormatter(v || 0)
      }
    ];

    processedData.series.forEach((pvData) => {
      seriesConfig.push({
        label: `${pvData.name} (${pvData.unit})`,
        stroke: pvData.pen.color,
        width: pvData.pen.lineWidth,
        points: {
          show: false
        },
        value: (u: any, v: number | null) => v?.toFixed(2) ?? 'N/A'
      });
    });

    const opts: uPlot.Options = {
      width: parentWidth,
      height: parentHeight,
      series: seriesConfig,
      plugins: [createTooltipPlugin()],
      cursor: {
        show: true,
        points: {
          show: false
        },
        drag: {
          setScale: false
        },
        sync: {
          key: 'epics-chart'
        }
      },
      scales: {
        x: {
          time: true,
          auto: true
        },
        y: {
          auto: false,
          range: (u, min, max) => {
            const [yMin, yMax] = processedData.yRange;
            return [yMin, yMax];
          }
        }
      },
      axes: [
        {
          scale: "x",
          side: 2,
          grid: { 
            show: true, 
            stroke: "rgba(0,0,0,0.05)",  // Lighter color
            width: 1,                     // Thinner lines
          },
          space: 40,
          rotate: -20,
          gap: 5,
          size: 30,
          splits: u => {                  // Custom tick spacing for X axis
            const rangeSecs = (u.data[0][u.data[0].length - 1] - u.data[0][0]) / 1000;
            let increment;
            
            if (rangeSecs <= 60) increment = 5;          // 5 second intervals
            else if (rangeSecs <= 300) increment = 15;   // 15 second intervals
            else if (rangeSecs <= 900) increment = 30;   // 30 second intervals
            else if (rangeSecs <= 3600) increment = 60;  // 1 minute intervals
            else increment = 300;                        // 5 minute intervals

            const splits = [];
            let t = Math.ceil(u.data[0][0] / 1000 / increment) * increment;
            const end = Math.floor(u.data[0][u.data[0].length - 1] / 1000);
            
            while (t <= end) {
              splits.push(t * 1000);
              t += increment;
            }
            
            return splits;
          }
        },
        {
          scale: "y",
          side: 3,
          grid: { 
            show: true, 
            stroke: "rgba(0,0,0,0.05)",  // Lighter color
            width: 1,                     // Thinner lines
          },
          ticks: { 
            show: true,
            stroke: "rgba(0,0,0,0.05)",  // Lighter color for ticks
            width: 1,
            size: 4,
          },
          splits: u => {                  // More Y-axis gridlines
            const [min, max] = (u.scales.y.range as (u: uPlot, min: number, max: number) => [number, number])(u, 0, 0) || [0, 0];
            const range = max - min;
            const step = range / 10;      // 10 divisions
            const splits = [];
            for (let i = 0; i <= 10; i++) {
              splits.push(min + step * i);
            }
            return splits;
          },
          size: 40,
          gap: 5,
          values: (u, vals) => vals.map(v => v.toFixed(2))
        }
      ],
      padding: [10, 8, 30, 8], // [top, right, bottom, left] - significantly reduced
    };

    try {
      const allData = [
        processedData.series[0].data[0],
        ...processedData.series.map(s => s.data[1])
      ];

      const typedData = allData.map(arr => new Float64Array(arr));
      uPlotInstance = new uPlot(opts, typedData, chartRef);
    } catch (error) {
      console.error('Error initializing chart:', error);
    }
  };

  onMount(() => {
    if (chartRef) {
      const resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(() => {
          if (uPlotInstance && chartRef?.parentElement) {
            const { width, height } = chartRef.parentElement.getBoundingClientRect();
            uPlotInstance.setSize({ width, height });
          }
        });
      });

      if (chartRef.parentElement) {
        resizeObserver.observe(chartRef.parentElement);
        requestAnimationFrame(initChart);
      }

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
    <div class="relative w-full h-full overflow-hidden">
      <div 
        ref={chartRef} 
        class="w-full h-full"
        style={{
          "min-width": "100%",
          "min-height": "100%"
        }}
      />
    </div>
  );
}