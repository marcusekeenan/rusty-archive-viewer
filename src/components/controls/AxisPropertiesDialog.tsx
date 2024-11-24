import { createSignal, createEffect } from 'solid-js';
import type { AxisConfig } from '../chart/types';

interface AxisPropertiesDialogProps {
  isOpen: boolean;
  onClose: () => void;
  axis?: AxisConfig;
  existingAxes: Map<string, AxisConfig>;
  onSave: (axis: AxisConfig) => void;
}

export function AxisPropertiesDialog(props: AxisPropertiesDialogProps) {
  let debounceTimeout: number | undefined;
  
  const [properties, setProperties] = createSignal<AxisConfig>(
    props.axis || {
      id: `axis_${Date.now()}`,
      egu: '',
      position: 'left',
      autoRange: true,
      range: { low: 0, high: 100 },
      pvs: new Set(),
    }
  );

  createEffect(() => {
    if (props.isOpen && props.axis) {
      setProperties({
        ...props.axis,
        autoRange: props.axis.autoRange ?? true
      });
    }
  });

  const updateProperty = <K extends keyof AxisConfig>(
    key: K,
    value: AxisConfig[K]
  ) => {
    const newProps = { ...properties(), [key]: value };
    setProperties(newProps);
    
    if (debounceTimeout) {
      clearTimeout(debounceTimeout);
    }
    
    debounceTimeout = window.setTimeout(() => {
      props.onSave(newProps);
    }, 150);
  };

  const handleRangeChange = (key: 'low' | 'high', value: string) => {
    const numValue = parseFloat(value);
    if (!isNaN(numValue)) {
      const newProps = {
        ...properties(),
        range: {
          ...properties().range!,
          [key]: numValue
        }
      };
      setProperties(newProps);
      
      if (debounceTimeout) {
        clearTimeout(debounceTimeout);
      }
      
      debounceTimeout = window.setTimeout(() => {
        props.onSave(newProps);
      }, 150);
    }
  };

  if (!props.isOpen) return null;

  return (
    <div class="fixed inset-0 z-50" onClick={(e) => e.stopPropagation()}>
      <div 
        class="fixed inset-0 bg-black/50" 
        onClick={(e) => {
          e.stopPropagation();
          if (debounceTimeout) {
            clearTimeout(debounceTimeout);
          }
          props.onClose();
        }}
      />

      <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] bg-white rounded-lg shadow-lg">
        <div class="flex items-center justify-between p-4 border-b">
          <h2 class="text-lg font-semibold">
            {props.axis ? 'Edit Axis' : 'New Axis'}
          </h2>
          <button 
            type="button"
            onClick={props.onClose}
            class="text-gray-500 hover:text-gray-700 text-2xl leading-none"
          >
            ×
          </button>
        </div>

        <div class="p-4 space-y-4">
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Engineering Units
            </label>
            <input
              type="text"
              value={properties().egu}
              onInput={(e) => updateProperty('egu', e.currentTarget.value)}
              class="w-full px-3 py-2 border rounded-md"
            />
          </div>

          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Position
            </label>
            <select
              value={properties().position}
              onChange={(e) => updateProperty('position', e.currentTarget.value as 'left' | 'right')}
              class="w-full px-3 py-2 border rounded-md"
            >
              <option value="left">Left</option>
              <option value="right">Right</option>
            </select>
          </div>

          <div class="flex items-center gap-2">
            <input
              type="checkbox"
              id="autoRange"
              checked={properties().autoRange}
              onChange={(e) => updateProperty('autoRange', e.currentTarget.checked)}
              class="rounded border-gray-300"
            />
            <label for="autoRange" class="text-sm font-medium text-gray-700">
              Auto Range
            </label>
          </div>

          {!properties().autoRange && (
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="block text-sm font-medium text-gray-700">
                  Min Value
                </label>
                <input
                  type="number"
                  value={properties().range?.low ?? 0}
                  onInput={(e) => handleRangeChange('low', e.currentTarget.value)}
                  class="w-full px-3 py-2 border rounded-md"
                />
              </div>
              <div class="space-y-2">
                <label class="block text-sm font-medium text-gray-700">
                  Max Value
                </label>
                <input
                  type="number"
                  value={properties().range?.high ?? 100}
                  onInput={(e) => handleRangeChange('high', e.currentTarget.value)}
                  class="w-full px-3 py-2 border rounded-md"
                />
              </div>
            </div>
          )}
          
          <div class="flex justify-end gap-2 mt-6">
            <button
              type="button"
              onClick={props.onClose}
              class="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}