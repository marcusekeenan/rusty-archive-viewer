// Core Protocol Types
export enum ProcessingMode {
  Raw = "Raw",  // Match the Rust enum variants exactly
  Optimized = "Optimized",
  Mean = "Mean",
  Max = "Max",
  Min = "Min",
  Jitter = "Jitter",
  StdDev = "StdDev",
  Count = "Count",
  FirstSample = "FirstSample",
  LastSample = "LastSample",
  FirstFill = "FirstFill",
  LastFill = "LastFill",
  Median = "Median",
  Variance = "Variance",
  PopVariance = "PopVariance",
  Kurtosis = "Kurtosis",
  Skewness = "Skewness",
  Linear = "Linear",
  Loess = "Loess",
  CAPlotBinning = "CAPlotBinning"
}

export interface ProcessingModeOption {
  value: ProcessingMode;
  label: string;
}

export const PROCESSING_MODE_OPTIONS: ProcessingModeOption[] = [
  { value: ProcessingMode.Optimized, label: "Optimized" },
  { value: ProcessingMode.Raw, label: "Raw" },
  { value: ProcessingMode.Mean, label: "Mean" },
  { value: ProcessingMode.Max, label: "Maximum" },
  { value: ProcessingMode.Min, label: "Minimum" },
  { value: ProcessingMode.Jitter, label: "Jitter" },
  { value: ProcessingMode.StdDev, label: "Standard Deviation" },
  { value: ProcessingMode.Count, label: "Count" },
  { value: ProcessingMode.FirstSample, label: "First Sample" },
  { value: ProcessingMode.LastSample, label: "Last Sample" },
  { value: ProcessingMode.FirstFill, label: "First Fill" },
  { value: ProcessingMode.LastFill, label: "Last Fill" },
  { value: ProcessingMode.Median, label: "Median" },
  { value: ProcessingMode.Variance, label: "Variance" },
  { value: ProcessingMode.PopVariance, label: "Population Variance" },
  { value: ProcessingMode.Kurtosis, label: "Kurtosis" },
  { value: ProcessingMode.Skewness, label: "Skewness" },
  { value: ProcessingMode.Linear, label: "Linear" },
  { value: ProcessingMode.Loess, label: "LOESS" },
  { value: ProcessingMode.CAPlotBinning, label: "CAPlot Binning" }
];


export enum DataFormat {
  Raw = "Raw",
  Json = "Json"
}

// Data Structures
export interface UPlotData {
  timestamps: number[];
  series: number[][];
  meta: Meta[];
}

export interface Meta {
  name: string;
  EGU?: string;
  [key: string]: string | undefined;
}

export interface Point {
  secs: number;
  nanos: number;
  val: PointValue;
  severity: number;
  status: number;
}

export type PointValue = 
  | { Float: number }
  | { Double: number }
  | { Int: number }
  | { Long: number }
  | { Short: number }
  | { Byte: number }
  | { String: string }
  | { Enum: number }
  | { ByteArray: number[] };

// PV & Visualization Types
export interface PVWithProperties {
  name: string;
  pen: PenProperties;
  metadata?: PVMetadata;
  axisId?: string;
}

export interface PenProperties {
  color: string;
  opacity: number;
  lineWidth: number;
  style: 'solid' | 'dashed' | 'dotted';
  showPoints: boolean;
  pointSize: number;
}

export interface PVMetadata {
  name: string;
  EGU?: string;
  PREC?: number;
  DESC?: string;
  LOPR?: number;
  HOPR?: number;
  DRVL?: number;
  DRVH?: number;
  LOW?: number;
  HIGH?: number;
  LOLO?: number;
  HIHI?: number;
}

// Chart & Axis Configuration
export interface AxisConfig {
  id: string;
  EGU: string;
  position: 'left' | 'right';
  autoRange: boolean;
  range?: AxisRange;
  pvs: Set<string>;
  color?: string;
}

export interface AxisRange {
  low: number;
  high: number;
}


export interface LiveModeConfig {
  enabled: boolean;
  mode: 'rolling' | 'append';
  updateInterval: number;
}

export interface TimeRange {
  start: Date;
  end: Date;
}

// Constants
export const DEFAULT_PEN_PROPERTIES: PenProperties = {
  color: '#2563eb',
  opacity: 1,
  lineWidth: 2,
  style: 'solid',
  showPoints: false,
  pointSize: 4,
};

 // Event & Callback Types
export interface AxisChangeEvent {
  axisId: string;
  changes: Partial<AxisConfig>;
}

export interface PVChangeEvent {
  pvName: string;
  changes: Partial<PVWithProperties>;
  axisId?: string;
}

export interface ChartUpdateEvent {
  pvs: PVWithProperties[];
  axes: Map<string, AxisConfig>;
  timeRange: TimeRange;
  data: UPlotData;
}

// Validation types
export interface ValidationResult {
  isValid: boolean;
  errors?: string[];
}

// Extended interfaces
export interface AxisUpdateOptions {
  autoAssign?: boolean;
  position?: 'left' | 'right';
  mergeExisting?: boolean;
}

// Utils
export interface DataPoint {
  timestamp: number;
  value: number;
  severity?: number;
  status?: number;
}