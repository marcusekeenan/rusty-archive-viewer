export type PenProperties = {
    color: string;
    width: number;
    style: 'solid' | 'dashed' | 'dotted';
    showPoints: boolean;
    pointSize: number;
    opacity: number;
  };
  
  export type PVWithProperties = {
    name: string;
    pen: PenProperties;
  };
  
  export const DEFAULT_PEN_PROPERTIES: PenProperties = {
    color: '#0066cc',
    width: 2,
    style: 'solid',
    showPoints: true,
    pointSize: 4,
    opacity: 1,
  };