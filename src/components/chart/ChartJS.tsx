// EPICSChart.tsx
import { onMount, createEffect, onCleanup } from 'solid-js';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  TimeScale,
  ChartData,
  ChartOptions,
  LineController,
} from 'chart.js';
import zoomPlugin from 'chartjs-plugin-zoom';
import 'chartjs-adapter-date-fns';
import type { PVWithProperties, PenProperties } from '../../../src/types'

// Register Chart.js components
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

type EPICSChartProps = {
  data: any[];
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  timezone: string;
  
};

interface ChartPoint {
  x: number;
  y: number;
}

// Helper to convert line style to dash pattern
const getLineStyle = (style: PenProperties['style']): number[] => {
  switch (style) {
    case 'dashed':
      return [8, 4];
    case 'dotted':
      return [2, 2];
    default:
      return [];
  }
};

export default function ChartJs(props: EPICSChartProps) {
  let chartRef: HTMLCanvasElement | undefined;
  let chartInstance: ChartJS | null = null;

  const processData = (rawData: any[]): ChartData<'line', ChartPoint[]> => {
    if (!rawData?.[0]?.data) {
      return { datasets: [] };
    }

    const datasets = rawData.map((pvData) => {
      const { data: points, meta, pen } = pvData;
      // Find matching PV info from props
      const pvInfo = props.pvs.find(pv => pv.name === meta.name);
      const currentPen = pvInfo?.pen || pen;

      const chartData: ChartPoint[] = points
        .filter((point: any) => point !== null)
        .map((point: any) => {
          const timestamp = point.timestamp || (point.secs * 1000);
          const value = point.value ?? point.val;
          
          if (typeof value !== 'number' || isNaN(value)) return null;
          
          return {
            x: timestamp,
            y: value
          };
        })
        .filter((p: ChartPoint | null): p is ChartPoint => p !== null);

      // Convert color to rgba for opacity support
      const rgbaColor = currentPen.color.startsWith('#') 
        ? `rgba(${parseInt(currentPen.color.slice(1, 3), 16)}, ${parseInt(currentPen.color.slice(3, 5), 16)}, ${parseInt(currentPen.color.slice(5, 7), 16)}, ${currentPen.opacity})`
        : currentPen.color;

      return {
        label: `${meta?.name ?? 'Unknown'} (${meta?.EGU ?? ''})`,
        data: chartData,
        // Line properties
        borderColor: rgbaColor,
        backgroundColor: rgbaColor,
        borderWidth: currentPen.lineWidth,
        borderDash: getLineStyle(currentPen.style),
        // Point properties
        pointRadius: currentPen.showPoints ? currentPen.pointSize : 0,
        pointHoverRadius: currentPen.showPoints ? currentPen.pointSize + 2 : 4,
        pointBackgroundColor: rgbaColor,
        pointBorderColor: rgbaColor,
        pointBorderWidth: 1,
        pointStyle: 'circle',
        // Other properties
        tension: 0.1,
        fill: false,
        cubicInterpolationMode: 'monotone' as const,
        hidden: !pvInfo
      };
    });

    return { datasets };
  };

  const createChart = (chartData: ChartData<'line', ChartPoint[]>) => {
    if (!chartRef) return;

    const options: ChartOptions<'line'> = {
  responsive: true,
  maintainAspectRatio: false,
  animation: false,
  layout: {
    padding: {
      top: 30,
      right: 20,
      bottom: 45,
      left: 20
    }
  },
  interaction: {
    mode: 'nearest',
    axis: 'x',
    intersect: false
  },
  plugins: {
    legend: {
      display: true,
      position: 'top',
      align: 'start',
      labels: {
        usePointStyle: true,
        pointStyle: 'circle',
        padding: 15,
        boxWidth: 8,
        boxHeight: 8,
        color: '#333',
        font: {
          size: 11
        }
      }
    },
    tooltip: {
      position: 'nearest' as const,
      mode: 'index',
      intersect: false,
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      titleColor: '#333',
      bodyColor: '#333',
      borderColor: '#ddd',
      borderWidth: 1,
      padding: 10,
      callbacks: {
        title: (context) => {
          if (!context.length) return '';
          const timestamp = context[0].parsed.x;
          return new Date(timestamp).toLocaleString();  // Display raw timestamp as received
        },
        label: (context) => {
          const value = context.parsed.y;
          const dataset = context.dataset;
          if (!dataset.hidden) {
            return `${dataset.label}: ${value.toFixed(2)}`;
          }
          return '';
        },
        labelColor: (context) => ({
          borderColor: context.dataset.borderColor as string,
          backgroundColor: context.dataset.borderColor as string,
        })
      }
    },
    zoom: {
      pan: {
        enabled: true,
        mode: 'x',
        modifierKey: 'shift',
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
          backgroundColor: 'rgba(127,127,127,0.2)',
          borderColor: 'rgba(127,127,127,0.4)',
          borderWidth: 1
        }
      },
      limits: {
        x: {
          min: 'original',
          max: 'original',
          minRange: 1000
        }
      }
    }
  },
  scales: {
    x: {
      type: 'time',
      time: {
        displayFormats: {
          millisecond: 'mm:ss.SSS',
          second: 'mm:ss',
          minute: 'HH:mm',
          hour: 'HH:mm',
          day: 'MMM D',
          week: 'MMM D',
          month: 'MMM YYYY',
          quarter: '[Q]Q YYYY',
          year: 'YYYY'
        }
      },
      grid: {
        color: 'rgba(0,0,0,0.05)',  // Lighter grid
        tickLength: 0  // Remove tick marks
      },
      ticks: {
        maxRotation: 0,
        autoSkip: true,
        maxTicksLimit: 8,
        padding: 8,
        font: {
          size: 11
        },
        major: {
          enabled: true
        },
        callback: (value) => {
          return new Date(value).toLocaleString();  // Display raw timestamp as received
        }
      },
      border: {
        display: false  // Remove axis line
      }
    },
    y: {
      beginAtZero: false,
      grid: {
        color: 'rgba(0,0,0,0.05)',  // Lighter grid
        tickLength: 0  // Remove tick marks
      },
      border: {
        display: false  // Remove axis line
      },
      ticks: {
        padding: 8,
        maxTicksLimit: 8,
        callback: (value) => typeof value === 'number' ? value.toFixed(2) : value,
        font: {
          size: 11
        }
      }
    }
  }
};

    

    chartInstance = new ChartJS(chartRef, {
      type: 'line',
      data: chartData,
      options
    });
  };

  const updateChart = () => {
    const processedData = processData(props.data);
  
    if (chartInstance) {
      const currentDatasets = chartInstance.data.datasets;
      const newDatasets = processedData.datasets;
  
      let hasChanges = false;
      newDatasets.forEach((dataset, i) => {
        const currentDataset = currentDatasets[i];
        if (!currentDataset || 
            dataset.data.length !== currentDataset.data.length || 
            !dataset.data.every((point, j) => {
              const currentPoint = currentDataset.data[j];
      
              // Check if currentPoint has x and y properties
              if (
                currentPoint &&
                typeof currentPoint === 'object' &&
                'x' in currentPoint &&
                'y' in currentPoint
              ) {
                return point.x === currentPoint.x && point.y === currentPoint.y;
              }
              return false;
            })) {
          hasChanges = true;
        }
      });
      
  
      if (hasChanges) {
        chartInstance.data = processedData;
        chartInstance.update('none');
      }
    } else {
      createChart(processedData);
    }
  };
  

  const handleResize = () => {
    if (chartInstance && chartRef?.parentElement) {
      chartInstance.resize();
    }
  };
  

  onMount(() => {
    if (chartRef) {
      const resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(handleResize);
      });

      if (chartRef.parentElement) {
        resizeObserver.observe(chartRef.parentElement);
        requestAnimationFrame(updateChart);
      }

      window.addEventListener('resize', handleResize);

      onCleanup(() => {
        resizeObserver.disconnect();
        window.removeEventListener('resize', handleResize);
        chartInstance?.destroy();
      });
    }
  });

  createEffect(() => {
    if (props.data) {
      updateChart();
    }
  });

  return (
    <div class="relative w-full h-full overflow-hidden bg-white">
      <canvas 
        ref={chartRef} 
        class="w-full h-full"
      />
      <div class="absolute bottom-1 left-0 right-0 bg-white/80 text-xs py-1 px-2 border-t">
        <div class="flex items-center gap-4">
          <div class="text-gray-600">⇧ + Drag to Pan</div>
          <div class="text-gray-600">Ctrl + Wheel to Zoom</div>
          <div class="text-gray-600">Drag to Box Zoom</div>
          <button
            class="ml-auto px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 text-xs transition-colors"
            onClick={() => chartInstance?.resetZoom()}
          >
            Reset Zoom
          </button>
        </div>
      </div>
    </div>
  );
}