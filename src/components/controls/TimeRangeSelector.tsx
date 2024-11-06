import { createSignal, createEffect, For, Show } from 'solid-js';

interface TimeRangeSelectorProps {
  disabled?: boolean;
  initialTimezone?: string; // Change to initialTimezone
  onChange?: (start: Date, end: Date, timezone: string) => void;
}

const TimeRangeSelector = (props: TimeRangeSelectorProps) => {
  const [startDate, setStartDate] = createSignal<Date>(new Date(Date.now() - 3600000));
  const [endDate, setEndDate] = createSignal<Date>(new Date());
  const [timezone, setTimezone] = createSignal(
    props.initialTimezone || Intl.DateTimeFormat().resolvedOptions().timeZone
  );
  const [relativeRange, setRelativeRange] = createSignal('1h');

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

  const formatForInput = (date: Date): string => {
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
      timeZone: timezone()
    }).replace(/(\d+)\/(\d+)\/(\d+), (\d+):(\d+):(\d+)/, '$3-$1-$2T$4:$5');
  };

  const formatForDisplay = (date: Date): string => {
    try {
      return new Intl.DateTimeFormat('en-US', {
        dateStyle: 'medium',
        timeStyle: 'medium',
        timeZone: timezone()
      }).format(date);
    } catch (error) {
      console.error('Date formatting error:', error);
      return 'Invalid date';
    }
  };

  const updateTimeRange = (start: Date, end: Date) => {
    setStartDate(start);
    setEndDate(end);
    props.onChange?.(start, end, timezone());
  };

  const handleRelativeRangeChange = (value: string) => {
    setRelativeRange(value);
    if (value === 'custom') return;

    const { start, end } = getRelativeTimeRange(value);
    updateTimeRange(start, end);
  };

  const handleDateInput = (isStart: boolean, value: string) => {
    try {
      const date = new Date(value);
      if (isStart) {
        updateTimeRange(date, endDate());
      } else {
        updateTimeRange(startDate(), date);
      }
      setRelativeRange('custom');
    } catch (error) {
      console.error('Invalid date input:', error);
    }
  };

  const handleTimezoneChange = (e: Event) => {
    const newTimezone = (e.target as HTMLSelectElement).value;
    setTimezone(newTimezone);
    
    if (relativeRange() !== 'custom') {
      const { start, end } = getRelativeTimeRange(relativeRange());
      updateTimeRange(start, end);
    } else {
      // If in custom mode, update with current dates
      updateTimeRange(startDate(), endDate());
    }
  };

  // Initialize with default range
  createEffect(() => {
    if (props.initialTimezone && props.initialTimezone !== timezone()) {
      setTimezone(props.initialTimezone);
    }
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
          disabled={props.disabled}
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
          onChange={(e) => handleRelativeRangeChange(e.target.value)}
          class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          disabled={props.disabled}
        >
          <For each={timeRanges}>
            {(range) => <option value={range.value}>{range.label}</option>}
          </For>
        </select>
      </div>

      {/* Custom Time Range Inputs */}
      <Show when={relativeRange() === 'custom'}>
        <div class="flex flex-col gap-2">
          <label class="font-medium">Start Time ({timezone()})</label>
          <input
            type="datetime-local"
            value={formatForInput(startDate())}
            onInput={(e) => handleDateInput(true, e.target.value)}
            disabled={props.disabled}
            class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 
                   disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>

        <div class="flex flex-col gap-2">
          <label class="font-medium">End Time ({timezone()})</label>
          <input
            type="datetime-local"
            value={formatForInput(endDate())}
            onInput={(e) => handleDateInput(false, e.target.value)}
            disabled={props.disabled}
            class="px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 
                   disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>
      </Show>

      {/* Current Range Display */}
      <div class="text-sm text-gray-600 mt-2">
        <div>Start: {formatForDisplay(startDate())}</div>
        <div>End: {formatForDisplay(endDate())}</div>
        <div>Timezone: {timezone()}</div>
      </div>
    </div>
  );
};

export default TimeRangeSelector;