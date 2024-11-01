import { createSignal, createEffect, For } from 'solid-js';
import type { ExtendedFetchOptions } from '../../utils/archiverApi';

type TimeRangeSelectorProps = {
  onChange: (start: Date, end: Date, options: ExtendedFetchOptions) => void;
  disabled?: boolean;
};

type TimeRangeOption = {
  value: string;
  label: string;
  operator: string | null;
  binSize?: number;  // bin size in seconds
};

const TimeRangeSelector = (props: TimeRangeSelectorProps) => {
  const [startDate, setStartDate] = createSignal('');
  const [endDate, setEndDate] = createSignal('');
  const [timezone, setTimezone] = createSignal(
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  const [relativeRange, setRelativeRange] = createSignal('1h');

  // Configuration options
  const timezones = [
    'UTC',
    'America/Los_Angeles',
    'America/Denver',
    'America/Chicago',
    'America/New_York',
    'Europe/London',
    'Europe/Paris',
    'Asia/Tokyo',
  ];

  // Updated time ranges with appropriate operators and bin sizes
  const timeRanges: TimeRangeOption[] = [
    {
      value: 'custom',
      label: 'Custom Range',
      operator: null
    },
    {
      value: '15m',
      label: 'Last 15 Minutes',
      operator: 'raw'  // Raw data for short ranges
    },
    {
      value: '30m',
      label: 'Last 30 Minutes',
      operator: 'raw'
    },
    {
      value: '1h',
      label: 'Last Hour',
      operator: 'mean',
      binSize: 10  // 10-second bins
    },
    {
      value: '3h',
      label: 'Last 3 Hours',
      operator: 'mean',
      binSize: 30  // 30-second bins
    },
    {
      value: '6h',
      label: 'Last 6 Hours',
      operator: 'mean',
      binSize: 60  // 1-minute bins
    },
    {
      value: '12h',
      label: 'Last 12 Hours',
      operator: 'mean',
      binSize: 120  // 2-minute bins
    },
    {
      value: '24h',
      label: 'Last 24 Hours',
      operator: 'mean',
      binSize: 300  // 5-minute bins
    },
    {
      value: '2d',
      label: 'Last 2 Days',
      operator: 'mean',
      binSize: 600  // 10-minute bins
    },
    {
      value: '7d',
      label: 'Last Week',
      operator: 'mean',
      binSize: 900  // 15-minute bins
    },
    {
      value: '30d',
      label: 'Last 30 Days',
      operator: 'mean',
      binSize: 3600  // 1-hour bins
    }
  ];

  const getRelativeTimeRange = (value: string) => {
    const now = new Date();
    const start = new Date(now);
    const match = value.match(/^(\d+)([mhd])$/);

    if (!match) return { start: now, end: now, options: {} as ExtendedFetchOptions };

    const [_, amount, unit] = match;
    const num = parseInt(amount);

    switch (unit) {
      case 'm':
        start.setMinutes(start.getMinutes() - num);
        break;
      case 'h':
        start.setHours(start.getHours() - num);
        break;
      case 'd':
        start.setDate(start.getDate() - num);
        break;
    }

    const timeRange = timeRanges.find((r) => r.value === value);
    const options: ExtendedFetchOptions = {
      operator: timeRange?.operator ?? undefined, // Convert null to undefined
      timezone: timezone(),
      chart_width: window.innerWidth,  // Dynamic chart width
    };

    // Add bin size if specified
    if (timeRange?.operator === 'mean' && timeRange?.binSize) {
      options.operator = `mean_${timeRange.binSize}`;
    }

    return { start, end: now, options };
};

  const formatForInput = (date: Date | null): string => {
    if (!date) return '';
    return date.toISOString().slice(0, 19);  // Format: YYYY-MM-DDTHH:mm:ss
  };

  const updateTimeRange = (start: Date, end: Date, options: ExtendedFetchOptions) => {
    setStartDate(formatForInput(start));
    setEndDate(formatForInput(end));

    if (props.onChange) {
      props.onChange(start, end, options);
    }
  };

  const handleRelativeRangeChange = (value: string) => {
    setRelativeRange(value);
    if (value === 'custom') return;

    const { start, end, options } = getRelativeTimeRange(value);
    updateTimeRange(start, end, options);
  };

  const handleDateInput = (isStart: boolean, value: string) => {
    if (isStart) {
      setStartDate(value);
      updateTimeRange(new Date(value), new Date(endDate()), {
        timezone: timezone(),
        operator: 'raw'  // Default to raw data for custom ranges
      });
    } else {
      setEndDate(value);
      updateTimeRange(new Date(startDate()), new Date(value), {
        timezone: timezone(),
        operator: 'raw'
      });
    }
    setRelativeRange('custom');
  };

  const handleTimezoneChange = (e: Event) => {
    const tz = (e.target as HTMLSelectElement).value;
    setTimezone(tz);
    
    // Update time range with new timezone
    if (relativeRange() !== 'custom') {
      const { start, end, options } = getRelativeTimeRange(relativeRange());
      updateTimeRange(start, end, { ...options, timezone: tz });
    }
  };

  // Initialize the time range
  createEffect(() => {
    const { start, end, options } = getRelativeTimeRange(relativeRange());
    updateTimeRange(start, end, options);
  });

  return (
    <div class="flex flex-col gap-4">
      {/* Timezone Selector */}
      <div class="flex flex-col gap-2">
        <label class="font-medium">Timezone</label>
        <select
          value={timezone()}
          onChange={handleTimezoneChange}
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <For each={timezones}>
            {(tz) => <option value={tz}>{tz.replace('_', ' ')}</option>}
          </For>
        </select>
      </div>

      {/* Time Range Selector */}
      <div class="flex flex-col gap-2">
        <label class="font-medium">Time Range</label>
        <select
          value={relativeRange()}
          onChange={(e) => handleRelativeRangeChange((e.target as HTMLSelectElement).value)}
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <For each={timeRanges}>
            {(range) => <option value={range.value}>{range.label}</option>}
          </For>
        </select>
      </div>

      {/* Start Time Input */}
      <div class="flex flex-col gap-2">
        <label class="font-medium">Start Time ({timezone()})</label>
        <input
          type="datetime-local"
          value={startDate()}
          onInput={(e) => handleDateInput(true, (e.target as HTMLInputElement).value)}
          disabled={props.disabled || relativeRange() !== 'custom'}
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* End Time Input */}
      <div class="flex flex-col gap-2">
        <label class="font-medium">End Time ({timezone()})</label>
        <input
          type="datetime-local"
          value={endDate()}
          onInput={(e) => handleDateInput(false, (e.target as HTMLInputElement).value)}
          disabled={props.disabled || relativeRange() !== 'custom'}
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Current Range Display */}
      <div class="text-sm text-gray-600 mt-2">
        Current range: {startDate()} to {endDate()} ({timezone()})
      </div>
    </div>
  );
};

export default TimeRangeSelector;