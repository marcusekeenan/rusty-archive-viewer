// types.ts

// ================ Core Data Types ================
export interface Meta {
    name: string;
    egu: string;
    description?: string;
    precision?: number;
    archive_parameters?: ArchiveParameters;
    display_limits?: DisplayLimits;
    alarm_limits?: AlarmLimits;
  }
  
  export interface ArchiveParameters {
    sampling_period: number;
    sampling_method: string;
    last_modified: string;
    operator?: string;
    buffer_size?: number;
  }
  
  export interface DisplayLimits {
    low: number;
    high: number;
  }
  
  export interface AlarmLimits {
    low: number;
    high: number;
    lolo: number;
    hihi: number;
  }
  
  // ================ Data Points ================
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
  
  export interface NormalizedPVData {
    meta: Meta;
    data: ProcessedPoint[];
    statistics?: Statistics;
  }
  
  // ================ Chart Configuration ================
  export interface ChartConfig {
    pvs: PVWithProperties[];
    timeRange: TimeRange;
    yAxes: YAxisConfig[];
    options: ChartOptions;
    displayMode: DisplayMode;
    liveMode: LiveModeConfig;
  }
  
  export interface ChartOptions {
    showLegend: boolean;
    showGrid: boolean;
    animation: boolean;
    panZoom: boolean;
    tooltips: boolean;
    decimation: DecimationConfig;
  }
  
  export interface DecimationConfig {
    enabled: boolean;
    threshold: number;
    algorithm: 'min-max' | 'lttb';
  }
  
  // ================ PV Types ================
  export interface PVWithProperties {
    name: string;
    pen: PenProperties;
    config?: PVConfig;
  }
  
  export interface PVConfig {
    displayName?: string;
    yAxis?: number;
    visible: boolean;
    decimation?: DecimationConfig;
    alarmEnabled?: boolean;
  }
  
  export interface PenProperties {
    color: string;
    opacity: number;
    lineWidth: number;
    style: LineStyle;
    showPoints: boolean;
    pointSize: number;
    displayName?: string;
    yAxis?: number;
    visible: boolean;
  }
  
  export type LineStyle = 'solid' | 'dashed' | 'dotted';
  
  // ================ Display Types ================
  export interface DisplayMode {
    type: DisplayModeType;
    settings: Record<string, unknown>;
  }
  
  export type DisplayModeType = typeof DISPLAY_MODES[number]['value'];
  
  // ================ Time and Range Types ================
  export interface TimeRange {
    start: Date;
    end: Date;
    mode: TimeRangeMode;
  }
  
  export type TimeRangeMode = 'absolute' | 'relative' | 'live';

  export interface TimeWindow {
    start: Date;
    end: Date;
    mode: TimeRangeMode;
  }

    // ================ Store State Types ================ TODO: FIX THIS
  export interface ArchiveStoreState {
    isLive: boolean;
    timeWindow: TimeWindow;
    chartConfig: ChartConfig;
    pvs: PVWithProperties[];
    selectedPVs: Set<string>;
    visiblePVs: Set<string>;
    data: NormalizedPVData[];
    error: string | null;
    isLoading: boolean;
    }
  
  export interface YAxisConfig {
    id: number;
    unit: string;
    position: 'left' | 'right';
    pvs: string[];
    range?: YAxisRange;
  }
  
  export interface YAxisRange {
    min: number;
    max: number;
    autoScale: boolean;
    padding?: number;
  }
  
  // ================ Live Mode Types ================
  export interface LiveModeConfig {
    enabled: boolean;
    updateInterval: number;
    bufferSize: number;
    dropOffMode: 'slide' | 'reset';
    timeWindow: number;
  }
  
  // ================ Saved Configuration Types ================
  export interface SavedConfig {
    id: string;
    name: string;
    timestamp: number;
    config: ChartConfig;
    metadata: ConfigMetadata;
  }
  
  export interface ConfigMetadata {
    created: number;
    modified: number;
    description?: string;
    tags?: string[];
  }
  
  // ================ Constants ================
  export const PV_COLORS = [
    "#2563eb", "#dc2626", "#16a34a", "#9333ea", "#ea580c",
    "#0891b2", "#db2777", "#854d0e", "#2E4053", "#7c3aed",
    "#059669", "#d97706", "#be123c", "#475569", "#6366f1",
    "#b45309"
  ] as const;
  
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
    { value: "std", label: "Standard Deviation" }
  ] as const;
  
  // ================ Default Values ================
  export const DEFAULT_PEN_PROPERTIES: PenProperties = {
    color: PV_COLORS[0],
    opacity: 1,
    lineWidth: 2,
    style: 'solid',
    showPoints: false,
    pointSize: 4,
    visible: true,
  };
  
  export const DEFAULT_LIVE_MODE_CONFIG: LiveModeConfig = {
    enabled: false,
    updateInterval: 1000,
    bufferSize: 3600,
    dropOffMode: 'slide',
    timeWindow: 3600000,
  };
  
  // ================ Type Guards ================
  export const isNormalizedPVData = (data: unknown): data is NormalizedPVData => {
    const d = data as NormalizedPVData;
    return Boolean(d && typeof d === 'object' && 'meta' in d && 'data' in d && Array.isArray(d.data));
  };
  
  export const isPVWithProperties = (pv: unknown): pv is PVWithProperties => {
    const p = pv as PVWithProperties;
    return Boolean(p && typeof p === 'object' && 'name' in p && 'pen' in p && typeof p.name === 'string');
  };
  
  // ================ Utility Functions ================
  export const getNextColor = (existingPVs: PVWithProperties[]): string => {
    const usedColors = new Set(existingPVs.map(pv => pv.pen.color));
    return PV_COLORS.find(color => !usedColors.has(color)) ?? 
           PV_COLORS[existingPVs.length % PV_COLORS.length];
  };