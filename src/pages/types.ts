// types.ts
import type { Component } from "solid-js";
import type { 
  ExtendedFetchOptions, 
  NormalizedPVData 
} from "../utils/archiverApi";

export interface TimeRange {
  start: Date;
  end: Date;
}

export interface DebugLog {
  timestamp: string;
  message: string;
  type: "info" | "error" | "debug" | "success";
  details?: string | null;
}

export interface RealTimeMode {
  enabled: boolean;
  updateInterval: number;
  lastTimestamp: number;
  bufferSize: number;
  operator: string;
}

export interface DebugDialogProps {
  isOpen: boolean;
  onClose: () => void;
  data: NormalizedPVData[];
}

export const DEBUG_LOG_LIMIT = 50;

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