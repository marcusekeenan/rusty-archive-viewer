// src/components/controls/common/hooks/useTimeRange.ts

import { createSignal, createEffect } from "solid-js";
import type { TimeRange, TimeRangeMode } from "../../../../types";

export interface UseTimeRangeOptions {
  initialRange?: TimeRange;
  initialTimezone?: string;
  onRangeChange?: (range: TimeRange) => void;
}

export interface UseTimeRangeReturn {
  timeRange: () => TimeRange;
  handleTimeRangeChange: (start: Date, end: Date, newTimezone?: string) => void;
  timezone: () => string;
  setTimeRange: (range: TimeRange) => void;
  setTimezone: (newTimezone: string) => void;
  mode: () => TimeRangeMode;
  setMode: (mode: TimeRangeMode) => void;
}

const DEFAULT_RANGE: TimeRange = {
  start: new Date(Date.now() - 3600000), // 1 hour ago
  end: new Date(),
  mode: 'absolute'
};

export function useTimeRange(options: UseTimeRangeOptions = {}): UseTimeRangeReturn {
  const {
    initialRange = DEFAULT_RANGE,
    initialTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone,
    onRangeChange
  } = options;

  // State
  const [timeRange, setTimeRange] = createSignal<TimeRange>(initialRange);
  const [timezone, setTimezone] = createSignal<string>(initialTimezone);
  const [mode, setMode] = createSignal<TimeRangeMode>(initialRange.mode || 'absolute');

  // Handle time range changes
  const handleTimeRangeChange = (start: Date, end: Date, newTimezone?: string) => {
    if (newTimezone) {
      setTimezone(newTimezone);
    }

    const newRange: TimeRange = {
      start,
      end,
      mode: mode()
    };

    setTimeRange(newRange);
    onRangeChange?.(newRange);
  };

  // Effect to validate and adjust time ranges
  createEffect(() => {
    const current = timeRange();
    
    // Ensure start is before end
    if (current.start > current.end) {
      const newRange = {
        ...current,
        start: new Date(current.end.getTime() - 3600000) // Default to 1 hour before end
      };
      setTimeRange(newRange);
      onRangeChange?.(newRange);
    }

    // For live mode, ensure end is current time
    if (mode() === 'live') {
      const now = new Date();
      if (current.end.getTime() !== now.getTime()) {
        const newRange = {
          ...current,
          end: now
        };
        setTimeRange(newRange);
        onRangeChange?.(newRange);
      }
    }
  });

  // Effect to handle timezone changes
  createEffect(() => {
    const current = timeRange();
    const tz = timezone();

    // Convert times to new timezone if needed
    const convertDate = (date: Date): Date => {
      const utc = date.getTime() + date.getTimezoneOffset() * 60000;
      const tzOffset = new Date(utc).toLocaleString('en-US', { timeZone: tz });
      return new Date(tzOffset);
    };

    const newRange = {
      ...current,
      start: convertDate(current.start),
      end: convertDate(current.end)
    };

    if (newRange.start.getTime() !== current.start.getTime() ||
        newRange.end.getTime() !== current.end.getTime()) {
      setTimeRange(newRange);
      onRangeChange?.(newRange);
    }
  });

  return {
    timeRange,
    handleTimeRangeChange,
    timezone,
    setTimeRange,
    setTimezone,
    mode,
    setMode
  };
}