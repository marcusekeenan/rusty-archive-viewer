// src/components/controls/types.ts

import type {
  PVWithProperties,
  TimeRange,
  ChartConfig,
  LiveModeConfig,
  SavedConfig,
  DisplayMode,
  YAxisConfig,
  PenProperties
} from '../../types';

// ================ Main Controls Props ================
export interface ControlsProps {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  isLive: () => boolean;
  timeRange: () => TimeRange;
  onAddPV: (pv: string) => void;
  onRemovePV: (pv: string) => void;
  onTimeRangeChange: (start: Date, end: Date) => void;
  onLiveModeToggle: () => void;
  onSaveConfig: () => void;
  onLoadConfig: (config: ChartConfig) => void;
}

// ================ Component Props ================
export interface ChartControlProps {
  config: () => ChartConfig;
  onConfigChange: (config: Partial<ChartConfig>) => void;
  onSave: () => void;
  onLoad: (config: ChartConfig) => void;
}

export interface ConfigurationControlProps {
  currentConfig: () => ChartConfig;
  savedConfigs: () => SavedConfig[];
  onSave: (config: SavedConfig) => void;
  onLoad: (config: ChartConfig) => void;
  onExport: (format: 'json' | 'yaml') => void;
  onImport: (file: File) => Promise<void>;
}

export interface LiveControlProps {
  isLive: () => boolean;
  config: () => LiveModeConfig;
  onToggle: () => void;
  onConfigChange: (config: Partial<LiveModeConfig>) => void;
}

export interface PVControlProps {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  onAddPV: (pv: string) => void;
  onRemovePV: (pv: string) => void;
  onUpdatePV: (pv: string, properties: Partial<PVWithProperties>) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
}

export interface TimeRangeControlProps {
  timeRange: () => TimeRange;
  isLive: () => boolean;
  disabled?: boolean;
  timezone: string;
  onTimeRangeChange: (start: Date, end: Date, timezone: string) => void;
}

// ================ Dialog Props ================
export interface ChartSettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  config: ChartConfig;
  onConfigChange: (config: Partial<ChartConfig>) => void;
  yAxes: YAxisConfig[];
  onYAxisChange: (axes: YAxisConfig[]) => void;
}

export interface SaveConfigDialogProps {
  isOpen: boolean;
  onClose: () => void;
  currentConfig: ChartConfig;
  onSave: (name: string, description?: string) => void;
}

export interface LiveSettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  config: LiveModeConfig;
  onConfigChange: (config: Partial<LiveModeConfig>) => void;
}

export interface PenPropertiesDialogProps {
  isOpen: boolean;
  onClose: () => void;
  pv: string;
  properties: PenProperties;
  onSave: (properties: PenProperties) => void;
}

// ================ List Component Props ================
export interface PVListProps {
  pvs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  onEdit: (pv: PVWithProperties) => void;
  onRemove: (pvName: string) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
}

export interface PVListItemProps {
  pv: PVWithProperties;
  isVisible: boolean;
  onEdit: () => void;
  onRemove: () => void;
  onToggleVisibility: (isVisible: boolean) => void;
}

// ================ Hook Interfaces ================
export interface UseTimeRangeOptions {
  defaultRange?: TimeRange;
  timezone?: string;
  onRangeChange?: (range: TimeRange) => void;
}

export interface UsePVManagementOptions {
  onPVChange?: (pvs: PVWithProperties[]) => void;
  maxPVs?: number;
  validatePV?: (pv: string) => Promise<boolean>;
}

export interface UseChartConfigOptions {
  defaultConfig?: Partial<ChartConfig>;
  autoSave?: boolean;
  storageKey?: string;
}

// ================ Time Range Types ================
export interface TimeRangePreset {
  label: string;
  value: string;
  range: {
    start: Date;
    end: Date;
  } | (() => { start: Date; end: Date });
}

// ================ Constants ================
export const DEFAULT_TIME_PRESETS: TimeRangePreset[] = [
  {
    label: 'Last 15 Minutes',
    value: '15m',
    range: () => ({
      start: new Date(Date.now() - 15 * 60 * 1000),
      end: new Date()
    })
  },
  {
    label: 'Last Hour',
    value: '1h',
    range: () => ({
      start: new Date(Date.now() - 60 * 60 * 1000),
      end: new Date()
    })
  },
  {
    label: 'Last 24 Hours',
    value: '24h',
    range: () => ({
      start: new Date(Date.now() - 24 * 60 * 60 * 1000),
      end: new Date()
    })
  }
];