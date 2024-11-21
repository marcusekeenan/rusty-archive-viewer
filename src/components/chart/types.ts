// types.ts or chart/types.ts
// src/components/chart/types.ts
import type { Meta, ProcessedPoint, NormalizedPVData } from "../../types/api";
import type { PVWithProperties } from "../controls/types";
import type { TimeRange } from "../../types/common";

export interface ChartProps {
  data: NormalizedPVData[];
  pvs: PVWithProperties[];
  timeRange: TimeRange;
  timezone: string;
  axes: Map<string, AxisConfig>;
}

export interface TimeseriesPoint {
  x: number;
  y: number;
}

export interface AxisRange {
  low: number;
  high: number;
}

export interface AxisConfig {
  id: string;
  egu: string;
  position: "left" | "right";
  autoRange: boolean;
  range?: AxisRange;
  pvs: Set<string>;
  color?: string;
}

export interface AxisAssignment {
  pvName: string;
  axisId: string;
  autoRange: boolean;
  range?: AxisRange;
}

export interface AxisRange {
  low: number;
  high: number;
}

export interface AxisConfig {
  id: string;
  egu: string;
  position: "left" | "right";
  autoRange: boolean;
  range?: AxisRange;
  pvs: Set<string>;
  color?: string;
}

export interface AxisAssignment {
  pvName: string;
  axisId: string;
  autoRange: boolean;
  range?: AxisRange;
}
