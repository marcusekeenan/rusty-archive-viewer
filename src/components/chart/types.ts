// src/components/chart/types.ts

import type { 
  PVData, 
  PVWithProperties, 
  AxisConfig,
  TimeRange 
} from '../../types';

export interface ChartProps {
  data: PVData[];
  pvs: PVWithProperties[];
  timeRange: TimeRange;
  timezone: string;
  axes: Map<string, AxisConfig>;
}

export interface TimeseriesPoint {
  x: number;
  y: number;
}

// Component-specific chart configuration types can be added here