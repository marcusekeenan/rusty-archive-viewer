import { PV_COLORS } from "../components/controls/colors";
import { Meta } from './api';

export interface PenProperties {
  color: string;
  opacity: number;
  lineWidth: number;
  style: 'solid' | 'dashed' | 'dotted';
  showPoints: boolean;
  pointSize: number;
}

export const DEFAULT_PEN_PROPERTIES: PenProperties = {
  color: PV_COLORS[0],
  opacity: 1,
  lineWidth: 2,
  style: 'solid',
  showPoints: false,
  pointSize: 4,
};

export interface PVMetadata {
  name: string;
  egu: string;
  precision?: number;
  description?: string;
  displayLimits?: {
    low: number;
    high: number;
  };
}

export interface PVWithProperties {
  name: string;
  pen: PenProperties;
  metadata?: PVMetadata | Meta;
  axisId?: string;
}

export interface AxisConfig {
  id: string;
  egu: string;
  position: 'left' | 'right';
  autoRange: boolean;
  range?: { low: number; high: number };
  pvs: Set<string>;
  color?: string;
}

export interface AxisAssignment {
  pvName: string;
  axisId: string;
  autoRange: boolean;
  range?: { low: number; high: number };
}