import { onMount, createEffect, onCleanup, createMemo } from "solid-js";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import type { PVWithProperties, AxisConfig, UPlotData, Meta } from "../../types";

interface ChartProps {
  data: UPlotData;
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  axes: Map<string, AxisConfig>;
}

export default function UPlotChart(props: ChartProps) {
  let chartEl: HTMLDivElement | undefined;
  let plot: uPlot | undefined;

  const createOptions = createMemo(() => {
    const options: uPlot.Options = {
      title: "",
      width: chartEl?.clientWidth || 800,
      height: chartEl?.clientHeight || 400,
      series: [
        {},  // time series (always first)
        {    // data series
          stroke: "blue",
          width: 1,
        }
      ],
      axes: [
        {},  // x-axis
        {}   // y-axis
      ]
    };
    
    return options;
  });

  createEffect(() => {
    const data = [
      props.data.timestamps,
      ...props.data.series
    ];

    try {
      if (plot) {
        plot.setData(data.map(arr => new Float64Array(arr)));
      } else if (chartEl) {
        const opts = createOptions();
        const typedData = data.map(arr => new Float64Array(arr));
        plot = new uPlot(opts, typedData, chartEl);
      }
    } catch (error) {
      console.error("Error with uPlot:", error);
    }
  });

  onCleanup(() => {
    plot?.destroy();
    plot = undefined;
  });

  return (
    <div class="relative w-full h-[400px]">
      <div ref={chartEl} class="w-full h-full"/>
    </div>
  );
}