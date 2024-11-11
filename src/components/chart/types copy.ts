// // components/chart/types.ts
// import type { 
//     NormalizedPVData,
//     YAxisConfig 
//   } from "../../utils/archiverApi";
//   import type { 
//     PVWithProperties 
//   } from "../controls/types";
  
//   export interface EPICSChartProps {
//     data: NormalizedPVData[];
//     pvs: PVWithProperties[];
//     timeRange: {
//       start: Date;
//       end: Date;
//     };
//     timezone: string;
//     yAxes?: YAxisConfig[];
//   }
  
//   // You might also want to add additional chart-specific types here
//   export interface ChartOptions {
//     showLegend?: boolean;
//     showGrid?: boolean;
//     animation?: boolean;
//     panZoom?: boolean;
//   }
  
//   export interface ChartDataPoint {
//     timestamp: number;
//     value: number;
//     min?: number;
//     max?: number;
//     stddev?: number;
//   }
  
//   export interface ChartSeriesOptions {
//     color: string;
//     lineWidth: number;
//     opacity: number;
//     style: 'solid' | 'dashed' | 'dotted';
//     showPoints: boolean;
//     pointSize: number;
//     yAxisId?: number;
//   }