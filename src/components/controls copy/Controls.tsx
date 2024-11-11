import { Component } from 'solid-js';
import { TimeRangeControl } from './TimeRangeControl';
import { PVControl } from './PVControl';
import { ChartControl } from './ChartControl';
import { LiveControl } from './LiveControl';
import type { ControlsProps } from './types';

export const Controls: Component<ControlsProps> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <LiveControl />
      <TimeRangeControl />
      <PVControl />
      <ChartControl />
    </div>
  );
};
