import { Component } from 'solid-js';
import { ChartControl } from '../ChartControl';
import { ConfigurationControl } from '../ConfigurationControl';
import { LiveControl } from '../LiveControl';
import { PVControl } from '../PVControl';
import { TimeRangeControl } from '../TimeRangeControl';

export const Controls: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <LiveControl />
      <TimeRangeControl />
      <PVControl />
      <ChartControl />
      <ConfigurationControl />
    </div>
  );
};

export default Controls;
