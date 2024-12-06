import { onMount, createEffect, onCleanup, createMemo } from "solid-js";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import type { PVWithProperties, AxisConfig, UPlotData } from "../../types";

interface ChartProps {
  data: UPlotData;
  pvs: PVWithProperties[];
  timeRange: { start: Date; end: Date };
  axes: Map<string, AxisConfig>;
}

export default function UPlotChart(props: ChartProps) {
  let chartEl: HTMLDivElement | undefined;
  let plot: uPlot | undefined;

  const getAxesConfig = () => {
    const axesConfig: uPlot.Axis[] = [{
      scale: "x",
      // Timestamps are already in milliseconds, just format them
      values: (u: uPlot, vals: number[]) => 
        vals.map(v => new Date(v).toLocaleString())
    }];
    
    const axisMap = new Map<string, number>();
    let leftCount = 0;
    let rightCount = 0;

    Array.from(props.axes.values()).forEach(axis => {
      const isLeft = axis.position === 'left';
      const scale = `scale_${axis.id}`;
      const axisIndex = axesConfig.length;
      axisMap.set(axis.id, axisIndex);

      axesConfig.push({
        scale,
        label: axis.EGU,
        side: isLeft ? 3 : 1,
        grid: { show: true },
        gap: 5,
        size: 80,
        show: true,
        space: (isLeft ? leftCount : rightCount) * 80,
      });

      isLeft ? leftCount++ : rightCount++;
    });

    return { axesConfig, axisMap };
  };

  const createOptions = createMemo(() => {
    const { axesConfig, axisMap } = getAxesConfig();
    
    const series: uPlot.Series[] = [
      {
        label: "Time",
        value: (u: uPlot, v: number) => new Date(v).toLocaleString()
      }
    ];

    props.data.meta.forEach((meta, idx) => {
      const pv = props.pvs.find(p => p.name === meta.name);
      if (!pv || !pv.axisId) return;

      const axisIdx = axisMap.get(pv.axisId);
      if (axisIdx === undefined) return;

      series.push({
        label: meta.name,
        scale: `scale_${pv.axisId}`,
        stroke: pv.pen.color,
        width: pv.pen.lineWidth,
        points: { 
          show: pv.pen.showPoints,
          size: pv.pen.pointSize 
        },
        dash: pv.pen.style === 'dashed' ? [10, 5] : 
              pv.pen.style === 'dotted' ? [2, 2] : undefined,
        value: (u: uPlot, v: number) => v.toFixed(3),
      });
    });

    const scales = {
      x: {
        time: true,
      },
      ...Object.fromEntries(
        Array.from(props.axes.values()).map(axis => [
          `scale_${axis.id}`,
          {
            auto: true,
            range: undefined,
            distr: 1,  // Linear distribution for data
          }
        ])
      )
    };

    return {
      width: chartEl?.clientWidth || 800,
      height: chartEl?.clientHeight || 400,
      scales,
      series,
      axes: axesConfig,
      cursor: {
        sync: { key: "archiver" },
        drag: { x: true, y: false },
        focus: {
          prox: 30,
        },
      },
    } as uPlot.Options;
  });

  createEffect(() => {
    // Data is already in correct format (timestamps in ms)
    const data = [
      new Float64Array(props.data.timestamps),
      ...props.data.series.map(arr => new Float64Array(arr))
    ];

    try {
      if (plot) {
        plot.setData(data);
        plot.setSize({ 
          width: chartEl?.clientWidth || 800,
          height: chartEl?.clientHeight || 400 
        });
      } else if (chartEl) {
        plot = new uPlot(createOptions(), data, chartEl);
      }
    } catch (error) {
      console.error("Error with uPlot:", error);
    }
  });

  onMount(() => {
    if (chartEl) {
      const resizeObserver = new ResizeObserver(() => {
        if (!plot) return;
        const width = chartEl?.clientWidth || 800;
        const height = chartEl?.clientHeight || 400;
        plot.setSize({ width, height });
      });
      resizeObserver.observe(chartEl);
      onCleanup(() => resizeObserver.disconnect());
    }
  });

  onCleanup(() => {
    plot?.destroy();
    plot = undefined;
  });

  return (
    <div class="relative w-full h-[400px]">
      <div ref={chartEl} class="w-full h-full" />
    </div>
  );
}