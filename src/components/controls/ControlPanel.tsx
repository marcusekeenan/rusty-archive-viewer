import {
  LiveModeConfig,
  ProcessingMode,
  PROCESSING_MODE_OPTIONS,
} from "../../types";

interface ControlPanelProps {
  liveModeConfig: () => {
    enabled: boolean;
    mode: "rolling" | "append";
    updateInterval: number;
  };
  processingMode: () => ProcessingMode;
  onLiveModeToggle: () => void;
  onLiveModeConfigChange: (config: Partial<LiveModeConfig>) => void;
  onProcessingModeChange: (mode: ProcessingMode) => void;
  onRefresh: () => void;
  onSaveConfig?: () => string;
  onLoadConfig?: (config: string) => void;
  loading: () => boolean;
}

export default function ControlPanel(props: ControlPanelProps) {
  return (
    <div class="bg-white rounded-lg shadow-sm p-4">
      <div class="flex items-center justify-between">
        <select
          value={props.processingMode()}
          onChange={(e) => {
            console.log('Selected mode:', e.target.value);
            console.log('Current mode:', props.processingMode());
            const mode = e.target.value as ProcessingMode;
            console.log('Mode to be set:', mode);
            props.onProcessingModeChange(mode);
          }}
          class="px-3 py-1.5 border rounded text-sm"
        >
          {PROCESSING_MODE_OPTIONS.map((mode) => (
            <option value={mode.value}>
              {mode.label}
            </option>
          ))}
        </select>

        <div class="flex items-center gap-4">
          {props.liveModeConfig().enabled && (
            <select
              value={props.liveModeConfig().mode}
              onChange={(e) =>
                props.onLiveModeConfigChange({
                  mode: e.target.value as "rolling" | "append",
                })
              }
              class="px-3 py-1.5 border rounded text-sm"
            >
              <option value="rolling">Rolling Window</option>
              <option value="append">Append</option>
            </select>
          )}

          <button
            onClick={props.onLiveModeToggle}
            class={`inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium 
              ${
                props.liveModeConfig().enabled
                  ? "bg-red-100 text-red-700 hover:bg-red-200"
                  : "bg-green-100 text-green-700 hover:bg-green-200"
              } transition-colors`}
          >
            {props.liveModeConfig().enabled ? (
              <>
                <div class="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
                <span>Live</span>
              </>
            ) : (
              <span>Go Live</span>
            )}
          </button>

          <button
            onClick={props.onRefresh}
            disabled={props.loading()}
            class="inline-flex items-center justify-center px-4 py-1.5 bg-blue-50 text-blue-700 
                   rounded-md text-sm font-medium hover:bg-blue-100 w-24
                   disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {props.loading() ? (
              <div class="w-4 h-4 border-2 border-blue-700 border-t-transparent rounded-full animate-spin" />
            ) : (
              <span>Refresh</span>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}