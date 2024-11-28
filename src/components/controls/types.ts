import { PV_COLORS } from "./colors";

export type PenProperties = {
    color: string;
    opacity: number;
    lineWidth: number;
    style: 'solid' | 'dashed' | 'dotted';
    showPoints: boolean;
    pointSize: number;
  };
  
  
  export const DEFAULT_PEN_PROPERTIES: PenProperties = {
    color: PV_COLORS[0], // Use first color as default
    opacity: 1,
    lineWidth: 2,
    style: 'solid',
    showPoints: false,
    pointSize: 4,
  };

  export interface PVMetadata {
    name: string;
    EGU: string;  // Engineering Units
    precision?: number;
    description?: string;
    displayLimits?: {
      low: number;
      high: number;
    };
  }
  
  export interface AxisConfig {
    id: string;
    EGU: string;
    position: 'left' | 'right';
    autoRange: boolean;
    range?: {
      min: number;
      max: number;
    };
    pvs: Set<string>;  // PVs using this axis
    color?: string;    // Optional color for the axis labels
  }
  
  // Extend existing PVWithProperties interface
  export interface PVWithProperties {
    name: string;
    pen: PenProperties;
    metadata?: PVMetadata;
    axisId?: string;   // Reference to which axis this PV uses
  }
  
  // For managing axis assignments
  export interface AxisAssignment {
    pvName: string;
    axisId: string;
    autoRange: boolean;
    range?: { min: number; max: number };
  }
 