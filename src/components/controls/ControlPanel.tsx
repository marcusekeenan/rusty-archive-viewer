import { LiveModeConfig, DataOperator } from "../../types";

interface ControlPanelProps {
  liveModeConfig: () => LiveModeConfig;
  dataOperator: () => DataOperator;
  onLiveModeToggle: () => void;
  onLiveModeConfigChange: (config: Partial<LiveModeConfig>) => void;
  onDataOperatorChange: (operator: DataOperator) => void;
  onRefresh: () => void;
  onExport: (type: 'visible' | 'raw') => void;
  loading: () => boolean;
}

export default function ControlPanel(props: ControlPanelProps) {
  return (
    <div class="bg-white rounded-lg shadow-sm p-4">
      <div class="flex items-center justify-between">
        <select
          value={props.dataOperator()}
          onChange={(e) => props.onDataOperatorChange(e.target.value as DataOperator)}
          class="px-3 py-1.5 border rounded text-sm"
        >
          {Object.values(DataOperator).map((op) => (
            <option value={op}>{op}</option>
          ))}
        </select>

        <div class="flex items-center gap-4">
          {props.liveModeConfig().enabled && (
            <select
              value={props.liveModeConfig().mode}
              onChange={(e) => props.onLiveModeConfigChange({ mode: e.target.value as 'rolling' | 'append' })}
              class="px-3 py-1.5 border rounded text-sm"
            >
              <option value="rolling">Rolling Window</option>
              <option value="append">Append</option>
            </select>
          )}

          <button
            onClick={props.onLiveModeToggle}
            class={`inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium 
              ${props.liveModeConfig().enabled
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
            class="inline-flex items-center gap-2 px-3 py-1.5 bg-blue-50 text-blue-700 
                   rounded-md text-sm font-medium hover:bg-blue-100 
                   disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {props.loading() ? (
              <>
                <div class="w-4 h-4 border-2 border-blue-700 border-t-transparent rounded-full animate-spin" />
                <span>Loading...</span>
              </>
            ) : (
              <span>Refresh</span>
            )}
          </button>

          <button
            onClick={() => props.onExport('visible')}
            class="px-3 py-1.5 bg-gray-100 text-gray-700 rounded-md text-sm font-medium hover:bg-gray-200"
          >
            Export Visible
          </button>

          <button
            onClick={() => props.onExport('raw')}
            class="px-3 py-1.5 bg-gray-100 text-gray-700 rounded-md text-sm font-medium hover:bg-gray-200"
          >
            Export Raw
          </button>
        </div>
      </div>
    </div>
  );
}