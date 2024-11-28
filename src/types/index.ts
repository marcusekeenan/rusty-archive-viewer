// src/types/index.ts

// ===============================
// Core Types (from Rust)
// ===============================

/**
 * Direct mapping of Rust types from types.rs
 */
export interface Meta {
    [key: string]: string;
  }
  
  export interface Point {
    secs: number;
    nanos: number;
    val: any;  // Maps to JsonValue in Rust
    severity: number;
    status: number;
  }
  
  export interface PVData {
    meta: Meta;
    data: Point[];
  }
  
  export interface Config {
    url: string;
    timeout_secs: number;
  }
  
  // ===============================
  // PV & Visualization Types
  // ===============================
  
  /**
   * Properties for styling PV visualization
   */
  export interface PenProperties {
    color: string;
    opacity: number;
    lineWidth: number;
    style: 'solid' | 'dashed' | 'dotted';
    showPoints: boolean;
    pointSize: number;
  }
  
  /**
   * Standardized PV metadata interface
   */
  export interface PVMetadata {
    name: string;
    EGU?: string;           // Engineering Units
    PREC?: number;          // Precision
    DESC?: string;          // Description
    LOPR?: number;          // Low Operating Range
    HOPR?: number;          // High Operating Range
    DRVL?: number;          // Drive Low Limit
    DRVH?: number;          // Drive High Limit
    LOW?: number;           // Low Alarm Limit
    HIGH?: number;          // High Alarm Limit
    LOLO?: number;          // Low Low Alarm Limit
    HIHI?: number;          // High High Alarm Limit
  }
  
  /**
   * PV with associated visualization properties
   */
  export interface PVWithProperties {
    name: string;
    pen: PenProperties;
    metadata?: PVMetadata;
    axisId?: string;
  }
  
  // ===============================
  // Chart & Axis Configuration
  // ===============================
  
  export interface AxisRange {
    low: number;
    high: number;
  }
  
  export interface AxisConfig {
    id: string;
    EGU: string;
    position: 'left' | 'right';
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
  
  // ===============================
  // Data Processing & Display
  // ===============================
  
  export enum DataOperator {
    Raw = 'raw',
    Optimized = 'optimized',
    FirstSample = 'firstSample',
    LastSample = 'lastSample',
    FirstFill = 'firstFill',
    LastFill = 'lastFill',
    Mean = 'mean',
    Min = 'min',
    Max = 'max',
    Count = 'count',
    Median = 'median',
    Std = 'std'
  }
  
  export interface ProcessedPoint {
    timestamp: number;
    severity: number;
    status: number;
    value: number;
    min: number;
    max: number;
    stddev: number;
    count: number;
  }
  
  export interface Statistics {
    mean: number;
    std_dev: number;
    min: number;
    max: number;
    count: number;
    first_timestamp: number;
    last_timestamp: number;
  }
  
  // ===============================
  // Application State & Configuration
  // ===============================
  
  export interface TimeRange {
    start: Date;
    end: Date;
  }
  
  export interface LiveModeConfig {
    enabled: boolean;
    mode: 'rolling' | 'append';
    updateInterval: number;
  }
  
  export interface DebugLog {
    timestamp: string;
    message: string;
    type: 'info' | 'error' | 'debug' | 'success';
    details?: string | null;
    source?: string;
  }
  
  export interface ExportConfig {
    format: 'csv' | 'json' | 'matlab' | 'text' | 'svg';
    type: 'visible' | 'raw';
  }
  
  // ===============================
  // Constants & Defaults
  // ===============================
  
  export const DEFAULT_PEN_PROPERTIES: PenProperties = {
    color: '#2563eb',
    opacity: 1,
    lineWidth: 2,
    style: 'solid',
    showPoints: false,
    pointSize: 4,
  };
  
  export const DEBUG_LOG_LIMIT = 50;
  
  export const DISPLAY_MODES = [
    { value: 'raw', label: 'Raw Data' },
    { value: 'firstSample', label: 'First Sample' },
    { value: 'lastSample', label: 'Last Sample' },
    { value: 'firstFill', label: 'First Fill (with interpolation)' },
    { value: 'lastFill', label: 'Last Fill (with interpolation)' },
    { value: 'mean', label: 'Mean Value' },
    { value: 'min', label: 'Minimum Value' },
    { value: 'max', label: 'Maximum Value' },
    { value: 'count', label: 'Sample Count' },
    { value: 'median', label: 'Median Value' },
    { value: 'std', label: 'Standard Deviation' }
  ] as const;
  
  // Type utilities
  export type DisplayMode = typeof DISPLAY_MODES[number]['value'];