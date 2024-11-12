// LiveModeControls.tsx
import { JSX, Show } from 'solid-js';

interface LiveModeControlsProps {
  mode: 'rolling' | 'append';  // Changed 'cumulative' to 'append'
  onModeChange: (mode: 'rolling' | 'append') => void;
  isLive: boolean;
  onLiveToggle: () => void;
}

export default function LiveModeControls(props: LiveModeControlsProps): JSX.Element {
  return (
    <div class="flex items-center gap-4">
      <button
        onClick={props.onLiveToggle}
        class={`inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium 
          ${props.isLive
            ? "bg-red-100 text-red-700 hover:bg-red-200"
            : "bg-green-100 text-green-700 hover:bg-green-200"
          } transition-colors`}
      >
        {props.isLive ? (
          <>
            <div class="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
            <span>Live</span>
          </>
        ) : (
          <span>Go Live</span>
        )}
      </button>

      <Show when={props.isLive}>
        <select
          value={props.mode}
          onChange={(e) => props.onModeChange(e.target.value as 'rolling' | 'append')}
          class="px-3 py-1.5 border rounded text-sm"
        >
          <option value="rolling">Rolling Window</option>
          <option value="append">Append</option>
        </select>
      </Show>
    </div>
  );
}