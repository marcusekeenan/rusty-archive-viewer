import { PV_COLORS } from "./colors";

export type PenProperties = {
    color: string;
    opacity: number;
    lineWidth: number;
    style: 'solid' | 'dashed' | 'dotted';
    showPoints: boolean;
    pointSize: number;
  };
  
  export type PVWithProperties = {
    name: string;
    pen: PenProperties;
  };
  
  export const DEFAULT_PEN_PROPERTIES: PenProperties = {
    color: PV_COLORS[0], // Use first color as default
    opacity: 1,
    lineWidth: 2,
    style: 'solid',
    showPoints: true,
    pointSize: 4,
  };

 