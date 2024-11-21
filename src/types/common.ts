export interface TimeRange {
    start: Date;
    end: Date;
  }
  
  export const DISPLAY_MODES = [
    { value: "raw", label: "Raw Data" },
    { value: "firstSample", label: "First Sample" },
    { value: "lastSample", label: "Last Sample" },
    { value: "firstFill", label: "First Fill (with interpolation)" },
    { value: "lastFill", label: "Last Fill (with interpolation)" },
    { value: "mean", label: "Mean Value" },
    { value: "min", label: "Minimum Value" },
    { value: "max", label: "Maximum Value" },
    { value: "count", label: "Sample Count" },
    { value: "median", label: "Median Value" },
    { value: "std", label: "Standard Deviation" },
  ] as const;
  
  export type ProcessingMode = (typeof DISPLAY_MODES)[number]["value"];
  
  export interface LiveModeConfig {
    enabled: boolean;
    mode: 'rolling' | 'append';
    updateInterval: number;
  }
  
  export interface RealTimeMode {
    enabled: boolean;
    updateInterval: number;
    lastTimestamp: number;
    bufferSize: number;
    operator: string;
  }