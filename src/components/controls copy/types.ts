import type {
  PVWithProperties,
  TimeRange,
  ChartConfig,
  LiveModeConfig
} from '../../types';

export interface ControlsProps {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  isLive: () => boolean;
  timeRange: () => TimeRange;
  liveMode: () => LiveModeConfig;
  onAddPV: (pv: string) => void;
  onRemovePV: (pv: string) => void;
  onTimeRangeChange: (start: Date, end: Date) => void;
  onLiveModeToggle: () => void;
  onSaveConfig: () => void;
  onLoadConfig: (config: ChartConfig) => void;
}

export interface TimeRangeControlProps {
  timeRange: () => TimeRange;
  disabled: () => boolean;
  onChange: (range: TimeRange) => void;
}

export interface PVControlProps {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  onAddPV: (pv: string) => void;
  onRemovePV: (pv: string) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
}

export interface ChartControlProps {
  onSave: () => void;
  onLoad: (config: ChartConfig) => void;
}

export interface LiveControlProps {
  isLive: () => boolean;
  config: () => LiveModeConfig;
  onToggle: () => void;
  onConfigChange: (config: Partial<LiveModeConfig>) => void;
}

export interface ConfigurationControlProps {
  onSave: () => void;
  onLoad: (config: ChartConfig) => void;
  onExport: () => void;
  onImport: () => void;
}
