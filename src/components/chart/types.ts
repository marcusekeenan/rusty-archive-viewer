// src/components/chart/types.ts

import type { 
  PVWithProperties, 
  AxisConfig,
  TimeRange,
  UPlotData
} from '../../types';

export interface ChartProps {
  data: UPlotData;
  pvs: PVWithProperties[];
  timeRange: TimeRange;
  axes: Map<string, AxisConfig>;
}

export interface TimeseriesPoint {
  x: number;
  y: number;
}

// Component-specific chart configuration types can be added here