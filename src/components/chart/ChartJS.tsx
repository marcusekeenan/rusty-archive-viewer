import { onMount, createEffect, onCleanup } from "solid-js";
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
  ScaleOptionsByType,
  LinearScaleOptions,
  CartesianScaleOptions,
  ChartArea,
  TickOptions,
} from "chart.js";
import zoomPlugin from "chartjs-plugin-zoom";
import "chartjs-adapter-date-fns";
import type {
  PVWithProperties,
  PenProperties,
  AxisConfig,
  AxisAssignment,
} from "../controls/types";

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
  axes: Map<string, AxisConfig>;
  onAxisChange?: (assignment: AxisAssignment) => void;
};

interface ChartPoint {
  x: number;
  y: number;
}

// Base configuration for tick options
const baseTickConfig = {
  backdropColor: "rgba(255, 255, 255, 0)",
  backdropPadding: 2 as number | ChartArea,
  callback: (value: number | string) =>
    typeof value === "number" ? value.toFixed(2) : String(value),
  display: true,
  color: "#333",
  font: {
    family: "'Helvetica Neue', 'Helvetica', 'Arial', sans-serif",
    size: 11,
    style: undefined,
    weight: undefined,
    lineHeight: undefined,
  },
  padding: 3,
  textStrokeColor: "rgba(255, 255, 255, 0)",
  textStrokeWidth: 0,
  z: 0,
} as const;

// Helper functions for tick configurations
const createLinearScaleTickConfig = (count: number) => ({
  ...baseTickConfig,
  count,
  format: {
    notation: "standard" as const,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  },
  precision: 2,
  stepSize: undefined,
  autoSkip: true,
  autoSkipPadding: 3,
  includeBounds: true,
  crossAlign: "near" as const,
  sampleSize: 8,
  maxTicksLimit: 11,
});

const createTimeScaleTickConfig = (count: number, timezone: string) => ({
  ...baseTickConfig,
  callback: (value: number | string) => {
    if (typeof value === "number") {
      return new Date(value).toLocaleString(undefined, {
        timeZone: timezone,
      });
    }
    return String(value);
  },
  display: true,
  autoSkip: true,
  autoSkipPadding: 3,
  maxTicksLimit: count,
});

// Helper to convert line style to dash pattern
const getLineStyle = (style: PenProperties["style"]): number[] => {
  switch (style) {
    case "dashed":
      return [8, 4];
    case "dotted":
      return [2, 2];
    default:
      return [];
  }
};

export default function ChartJs(props: EPICSChartProps) {
  let chartRef: HTMLCanvasElement | undefined;
  let chartInstance: ChartJS | null = null;

  const processData = (rawData: any[]): ChartData<"line", ChartPoint[]> => {
    if (!rawData?.[0]?.data) {
      return { datasets: [] };
    }

    const datasets = rawData.map((pvData) => {
      const { data: points, meta, pen } = pvData;
      const pvInfo = props.pvs.find((pv) => pv.name === meta.name);
      const currentPen = pvInfo?.pen || pen;

      const chartData: ChartPoint[] = points
        .filter((point: any) => point !== null)
        .map((point: any) => {
          const timestamp = point.timestamp || point.secs * 1000;
          const value = point.value ?? point.val;

          if (typeof value !== "number" || isNaN(value)) return null;

          return {
            x: timestamp,
            y: value,
          };
        })
        .filter((p: ChartPoint | null): p is ChartPoint => p !== null);

      const rgbaColor = currentPen.color.startsWith("#")
        ? `rgba(${parseInt(currentPen.color.slice(1, 3), 16)}, ${parseInt(currentPen.color.slice(3, 5), 16)}, ${parseInt(currentPen.color.slice(5, 7), 16)}, ${currentPen.opacity})`
        : currentPen.color;

      const axisId = pvInfo?.axisId || "default";

      return {
        label: `${meta?.name ?? "Unknown"} (${meta?.EGU ?? ""})`,
        data: chartData,
        yAxisID: axisId,
        borderColor: rgbaColor,
        backgroundColor: rgbaColor,
        borderWidth: currentPen.lineWidth,
        borderDash: getLineStyle(currentPen.style),
        pointRadius: currentPen.showPoints ? currentPen.pointSize : 0,
        pointHoverRadius: currentPen.showPoints ? currentPen.pointSize + 2 : 4,
        pointBackgroundColor: rgbaColor,
        pointBorderColor: rgbaColor,
        pointBorderWidth: 1,
        pointStyle: "circle",
        tension: 0.1,
        fill: false,
        cubicInterpolationMode: "monotone" as const,
        hidden: !pvInfo,
      };
    });

    return { datasets };
  };

  const createChart = (chartData: ChartData<"line", ChartPoint[]>) => {
    if (!chartRef) return;

    // Generate Y axis configurations from axis map
    const yAxes = Array.from(props.axes.entries()).reduce(
      (acc, [id, axis]) => {
        // @ts-ignore
        const scaleOptions: ScaleOptionsByType<"linear"> &
          CartesianScaleOptions = {
          type: "linear",
          display: true,
          position: axis.position as "left" | "right",
          bounds: "data",
          beginAtZero: false,
          offset: false,
          stack: undefined,
          reverse: false,
          grace: 0,
          title: {
            display: true,
            text: axis.egu,
            font: {
              size: 12,
              weight: "normal",
              family: "'Helvetica Neue', 'Helvetica', 'Arial', sans-serif",
            },
            align: "center",
            color: "#666",
            padding: 0,
          },
          suggestedMin: axis.autoRange ? undefined : axis.range?.min,
          suggestedMax: axis.autoRange ? undefined : axis.range?.max,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.05)",
            lineWidth: 1,
            drawTicks: false,
            offset: false,
            z: 0,
          },
          border: {
            display: false,
            dash: [],
            dashOffset: 0,
            width: 1,
            color: "rgba(0,0,0,0.1)",
            z: 0,
          },
          // ticks: createLinearScaleTickConfig(8)
        };

        acc[id] = scaleOptions;
        return acc;
      },
      {} as Record<string, ScaleOptionsByType<"linear">>
    );

    // Add default Y axis if none exist
    if (Object.keys(yAxes).length === 0) {
      // @ts-ignore
      const defaultScaleOptions: ScaleOptionsByType<"linear"> = {
        type: "linear",
        display: true,
        position: "left",
        bounds: "data",
        beginAtZero: false,
        offset: false,
        stack: undefined,
        reverse: false,
        grace: 0,
        title: {
          display: false,
          text: "",
          align: "center",
          color: "#666",
          font: {
            size: 11,
            weight: "normal",
            family: "'Helvetica Neue', 'Helvetica', 'Arial', sans-serif",
          },
          padding: 0,
        },
        grid: {
          display: true,
          color: "rgba(0,0,0,0.05)",
          lineWidth: 1,
          drawTicks: false,
          offset: false,
          z: 0,
        },
        border: {
          display: false,
          dash: [],
          dashOffset: 0,
          width: 1,
          color: "rgba(0,0,0,0.1)",
          z: 0,
        },
        // ticks: createLinearScaleTickConfig(8)
      };

      yAxes.default = defaultScaleOptions;
    }

    const timeScale = {
      type: "time" as const,
      display: true,
      offset: true,
      time: {
        displayFormats: {
          millisecond: "mm:ss.SSS",
          second: "mm:ss",
          minute: "HH:mm",
          hour: "HH:mm",
          day: "MMM D",
          week: "MMM D",
          month: "MMM YYYY",
          quarter: "[Q]Q YYYY",
          year: "YYYY",
        },
      },
      grid: {
        display: true,
        color: "rgba(0,0,0,0.05)",
        lineWidth: 1,
        drawTicks: true,
        offset: true,
        z: 0,
      },
      border: {
        display: false,
        dash: [],
        dashOffset: 0,
        width: 1,
        color: "rgba(0,0,0,0.1)",
        z: 0,
      },
      ticks: createTimeScaleTickConfig(8, props.timezone),
    };

    const options: ChartOptions<"line"> = {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      layout: {
        padding: {
          top: 30,
          right: 20,
          bottom: 45,
          left: 20,
        },
      },
      interaction: {
        mode: "nearest",
        axis: "x",
        intersect: false,
      },
      plugins: {
        legend: {
          display: true,
          position: "top",
          align: "start",
          labels: {
            usePointStyle: true,
            pointStyle: "circle",
            padding: 15,
            boxWidth: 8,
            boxHeight: 8,
            color: "#333",
            font: {
              size: 11,
              family: "'Helvetica Neue', 'Helvetica', 'Arial', sans-serif",
            },
          },
        },
        tooltip: {
          position: "nearest" as const,
          mode: "index",
          intersect: false,
          backgroundColor: "rgba(255, 255, 255, 0.95)",
          titleColor: "#333",
          bodyColor: "#333",
          borderColor: "#ddd",
          borderWidth: 1,
          padding: 10,
          callbacks: {
            title: (context) => {
              if (!context.length) return "";
              const timestamp = context[0].parsed.x;
              return new Date(timestamp).toLocaleString(undefined, {
                timeZone: props.timezone,
              });
            },
            label: (context) => {
              const value = context.parsed.y;
              const dataset = context.dataset;
              if (!dataset.hidden) {
                const axis = props.axes.get(dataset.yAxisID || "default");
                const units = axis?.egu || "";
                return `${dataset.label}: ${value.toFixed(2)} ${units}`;
              }
              return "";
            },
            labelColor: (context) => ({
              borderColor: context.dataset.borderColor as string,
              backgroundColor: context.dataset.borderColor as string,
            }),
          },
        },
        zoom: {
          pan: {
            enabled: true,
            mode: "x",
            modifierKey: "shift",
          },
          zoom: {
            wheel: {
              enabled: true,
              modifierKey: "ctrl",
            },
            pinch: {
              enabled: true,
            },
            mode: "x",
            drag: {
              enabled: true,
              backgroundColor: "rgba(127,127,127,0.2)",
              borderColor: "rgba(127,127,127,0.4)",
              borderWidth: 1,
            },
          },
          limits: {
            x: {
              min: "original",
              max: "original",
              minRange: 1000,
            },
          },
        },
      },
      scales: {
        x: timeScale,
        ...yAxes,
      },
    };

    chartInstance = new ChartJS(chartRef, {
      type: "line",
      data: chartData,
      options,
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
        if (
          !currentDataset ||
          dataset.data.length !== currentDataset.data.length ||
          !dataset.data.every((point, j) => {
            const currentPoint = currentDataset.data[j];

            if (
              currentPoint &&
              typeof currentPoint === "object" &&
              "x" in currentPoint &&
              "y" in currentPoint
            ) {
              return point.x === currentPoint.x && point.y === currentPoint.y;
            }
            return false;
          })
        ) {
          hasChanges = true;
        }
      });

      if (hasChanges) {
        chartInstance.data = processedData;
        chartInstance.update("none");
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

      window.addEventListener("resize", handleResize);

      onCleanup(() => {
        resizeObserver.disconnect();
        window.removeEventListener("resize", handleResize);
        chartInstance?.destroy();
      });
    }
  });

  createEffect(() => {
    if (props.data || props.axes) {
      updateChart();
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
            onClick={() => chartInstance?.resetZoom()}
          >
            Reset Zoom
          </button>
        </div>
      </div>
    </div>
  );
}
