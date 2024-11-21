// src/types/api.ts

export interface NormalizedPVData {
    meta: Meta;
    data: ProcessedPoint[];
    statistics?: Statistics;
  }
  
  export interface PointValue {
    secs: number;
    nanos?: number;
    val: number | number[] | string | Uint8Array;
    severity?: number;
    status?: number;
  }
  
  export interface Meta {
    name: string;
    egu: string;
    description?: string;
    precision?: number;
    display_high?: number;
    display_low?: number;
    drive_high?: number;
    drive_low?: number;
    alarm_high?: number;
    alarm_low?: number;
    alarm_hihi?: number;
    alarm_lolo?: number;
    archive_parameters?: {
      sampling_period: number;
      sampling_method: string;
      last_modified: string;
    };
    display_limits?: {
      low: number;
      high: number;
    };
    alarm_limits?: {
      low?: number;
      high?: number;
      lolo?: number;
      hihi?: number;
    };
    num_elements?: number;
    archive_deadband?: number;
    monitor_deadband?: number;
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
  
  export interface DetailedPVStatus {
    name: string;
    connected: boolean;
    archived: boolean;
    archive_enabled: boolean;
    last_event_time?: number;
    last_status?: string;
    error_count: number;
    last_error?: string;
    sampling_period?: number;
    sampling_method?: string;
    archival_state: string;
    last_modified?: string;
  }
  
  export interface LiveUpdateConfig {
    pvs: string[];
    updateIntervalMs: number;
    timezone?: string;
    onData: (data: Record<string, PointValue>) => void;
    onError?: (error: string) => void;
  }

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
    Ncount = 'ncount',
    Nth = 'nth',
    Median = 'median',
    Std = 'std',
    Variance = 'variance',
    PopVariance = 'popvariance',
    Jitter = 'jitter',
    Kurtosis = 'kurtosis',
    Skewness = 'skewness',
    IgnoreFlyers = 'ignoreflyers',
    Flyers = 'flyers'
  }
  
  // Optional helper to get display names for operators
  export const getOperatorDisplayName = (operator: DataOperator): string => {
    switch (operator) {
      case DataOperator.Raw: return 'Raw Data';
      case DataOperator.Optimized: return 'Optimized';
      case DataOperator.FirstSample: return 'First Sample';
      case DataOperator.LastSample: return 'Last Sample';
      case DataOperator.FirstFill: return 'First Fill';
      case DataOperator.LastFill: return 'Last Fill';
      case DataOperator.Mean: return 'Mean';
      case DataOperator.Min: return 'Minimum';
      case DataOperator.Max: return 'Maximum';
      case DataOperator.Count: return 'Count';
      case DataOperator.Ncount: return 'N-Count';
      case DataOperator.Nth: return 'Nth Value';
      case DataOperator.Median: return 'Median';
      case DataOperator.Std: return 'Standard Deviation';
      case DataOperator.Variance: return 'Variance';
      case DataOperator.PopVariance: return 'Population Variance';
      case DataOperator.Jitter: return 'Jitter';
      case DataOperator.Kurtosis: return 'Kurtosis';
      case DataOperator.Skewness: return 'Skewness';
      case DataOperator.IgnoreFlyers: return 'Ignore Flyers';
      case DataOperator.Flyers: return 'Flyers Only';
      default: return operator;
    }
  };
  
  // Optional grouping of operators by category
  export const operatorGroups = {
    basic: [
      DataOperator.Raw,
      DataOperator.Optimized,
      DataOperator.FirstSample,
      DataOperator.LastSample
    ],
    statistical: [
      DataOperator.Mean,
      DataOperator.Median,
      DataOperator.Min,
      DataOperator.Max,
      DataOperator.Std,
      DataOperator.Variance,
      DataOperator.PopVariance
    ],
    advanced: [
      DataOperator.Jitter,
      DataOperator.Kurtosis,
      DataOperator.Skewness,
      DataOperator.IgnoreFlyers,
      DataOperator.Flyers
    ],
    special: [
      DataOperator.Count,
      DataOperator.Ncount,
      DataOperator.Nth
    ]
  } as const;

export enum DataFormat {
    Json = 'json',
    Csv = 'csv',
    Raw = 'raw',
    Matlab = 'mat',
    Text = 'txt',
    Svg = 'svg'
}

export interface ExtendedFetchOptions {
    mode?: 'fixed' | 'rolling' | 'append';
    optimization?: 'raw' | 'auto' | 'optimized';
    operator?: DataOperator;
    targetPoints?: number;
    timezone?: string;
    format?: DataFormat;
    useRawProcessing?: boolean;
    fetchLatestMetadata?: boolean;
    doNotChunk?: boolean;
    batchSize?: number;
    retiredPvTemplate?: string;
    caCount?: number;
    caHow?: number;
}

// export interface TimeRange {
//     start: Date;
//     end: Date;
//   }
  
//   export type TimeRangeMode = 
//     | { type: 'Fixed'; start: number; end: number }
//     | { type: 'Rolling'; duration: number; end?: number }
//     | { type: 'Live'; baseMode: TimeRangeMode; lastUpdate: number };