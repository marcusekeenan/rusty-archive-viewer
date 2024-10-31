// TimeRangeSelector.tsx

import { createSignal, createEffect, For } from 'solid-js';

type TimeRangeSelectorProps = {
  onChange: (start: Date, end: Date, operator: string | null) => void;
  onTimezoneChange?: (timezone: string) => void;
  disabled?: boolean;
};

type TimeRangeOption = {
  value: string;
  label: string;
  operator: string | null;
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

  const timeRanges: TimeRangeOption[] = [
    {
      value: 'custom',
      label: 'Custom Range',
      operator: null,
    },
    {
      value: '15m',
      label: 'Last 15 Minutes',
      operator: null, // Raw data for short ranges
    },
    {
      value: '30m',
      label: 'Last 30 Minutes',
      operator: null,
    },
    {
      value: '1h',
      label: 'Last Hour',
      operator: 'optimized_360', // 10-second resolution
    },
    {
      value: '3h',
      label: 'Last 3 Hours',
      operator: 'optimized_1080', // 10-second resolution
    },
    {
      value: '6h',
      label: 'Last 6 Hours',
      operator: 'optimized_720', // 30-second resolution
    },
    {
      value: '12h',
      label: 'Last 12 Hours',
      operator: 'optimized_720',
    },
    {
      value: '24h',
      label: 'Last 24 Hours',
      operator: 'optimized_1440', // 1-minute resolution
    },
    {
      value: '2d',
      label: 'Last 2 Days',
      operator: 'optimized_2880',
    },
    {
      value: '7d',
      label: 'Last Week',
      operator: 'optimized_2016', // 5-minute resolution with statistics
    },
    {
      value: '30d',
      label: 'Last 30 Days',
      operator: 'optimized_4320', // 10-minute resolution with statistics
    },
  ];

  const getRelativeTimeRange = (value: string) => {
    const now = new Date();
    const start = new Date(now);
    const match = value.match(/^(\d+)([mhd])$/);

    if (!match) return { start: now, end: now, operator: null };

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

    return {
      start,
      end: now,
      operator: timeRanges.find((r) => r.value === value)?.operator || null,
    };
  };

  const formatForInput = (date: Date | null): string => {
    if (!date) return '';
    const d = new Date(date);
    return (
      d.getFullYear() +
      '-' +
      String(d.getMonth() + 1).padStart(2, '0') +
      '-' +
      String(d.getDate()).padStart(2, '0') +
      'T' +
      String(d.getHours()).padStart(2, '0') +
      ':' +
      String(d.getMinutes()).padStart(2, '0') +
      ':' +
      String(d.getSeconds()).padStart(2, '0')
    );
  };

  const updateTimeRange = (start: Date, end: Date, operator: string | null = null) => {
    setStartDate(formatForInput(start));
    setEndDate(formatForInput(end));

    if (props.onChange) {
      props.onChange(start, end, operator);
    }
  };

  const handleRelativeRangeChange = (value: string) => {
    setRelativeRange(value);
    if (value === 'custom') return;

    const { start, end, operator } = getRelativeTimeRange(value);
    updateTimeRange(start, end, operator);
  };

  const handleDateInput = (isStart: boolean, value: string) => {
    if (isStart) {
      setStartDate(value);
      updateTimeRange(new Date(value), new Date(endDate()));
    } else {
      setEndDate(value);
      updateTimeRange(new Date(startDate()), new Date(value));
    }
    setRelativeRange('custom');
  };

  const handleTimezoneChange = (e: Event) => {
    const tz = (e.target as HTMLSelectElement).value;
    setTimezone(tz);
    if (props.onTimezoneChange) {
      props.onTimezoneChange(tz);
    }
  };

  // Initialize the time range
  createEffect(() => {
    const { start, end, operator } = getRelativeTimeRange(relativeRange());
    updateTimeRange(start, end, operator);
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
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
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
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
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
