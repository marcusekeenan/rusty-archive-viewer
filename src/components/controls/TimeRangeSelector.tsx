import { createSignal, createEffect, For } from 'solid-js';
import type { ExtendedFetchOptions } from '../../utils/archiverApi';

type TimeRangeSelectorProps = {
  onChange: (start: Date, end: Date, options: ExtendedFetchOptions) => void;
  disabled?: boolean;
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

  const timeRanges = [
    { value: 'custom', label: 'Custom Range' },
    { value: '15m', label: 'Last 15 Minutes' },
    { value: '30m', label: 'Last 30 Minutes' },
    { value: '1h', label: 'Last Hour' },
    { value: '3h', label: 'Last 3 Hours' },
    { value: '6h', label: 'Last 6 Hours' },
    { value: '12h', label: 'Last 12 Hours' },
    { value: '24h', label: 'Last 24 Hours' },
    { value: '2d', label: 'Last 2 Days' },
    { value: '7d', label: 'Last Week' },
    { value: '30d', label: 'Last 30 Days' }
  ];

  const getRelativeTimeRange = (value: string) => {
    const now = new Date();
    const start = new Date(now);
    const match = value.match(/^(\d+)([mhd])$/);

    if (!match) return { start: now, end: now };

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

    return { start, end: now };
  };

  const formatForInput = (date: Date | null): string => {
    if (!date) return '';
    return date.toISOString().slice(0, 19);  // Format: YYYY-MM-DDTHH:mm:ss
  };

  const updateTimeRange = (start: Date, end: Date) => {
    setStartDate(formatForInput(start));
    setEndDate(formatForInput(end));

    if (props.onChange) {
      const options: ExtendedFetchOptions = {
        timezone: timezone(),
      };
      props.onChange(start, end, options);
    }
  };

  const handleRelativeRangeChange = (value: string) => {
    setRelativeRange(value);
    if (value === 'custom') return;

    const { start, end } = getRelativeTimeRange(value);
    updateTimeRange(start, end);
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
    
    // Update time range with new timezone if using a relative range
    if (relativeRange() !== 'custom') {
      const { start, end } = getRelativeTimeRange(relativeRange());
      updateTimeRange(start, end);
    }
  };

  // Initialize the time range
  createEffect(() => {
    const { start, end } = getRelativeTimeRange(relativeRange());
    updateTimeRange(start, end);
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
          disabled={props.disabled}
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
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
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
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
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