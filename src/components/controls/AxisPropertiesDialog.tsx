import { createSignal, createEffect, Show } from 'solid-js';
import type { AxisConfig, PVWithProperties } from '../../types';

interface AxisPropertiesDialogProps {
  isOpen: boolean;
  onClose: () => void;
  axis?: AxisConfig;
  existingAxes: Map<string, AxisConfig>;
  pvs: PVWithProperties[];
  currentAutoRange?: { low: number; high: number };
  onSave: (axis: AxisConfig) => void;
}

export function AxisPropertiesDialog(props: AxisPropertiesDialogProps) {
  let debounceTimeout: number | undefined;
  
  // Helper to get EGU from any assigned PV
  const getInitialEGU = () => {
    if (props.axis?.EGU) return props.axis.EGU;
    
    const axisId = props.axis?.id;
    if (axisId) {
      const axis = props.existingAxes.get(axisId);
      if (axis?.pvs.size) {
        for (const pvName of axis.pvs) {
          const pv = props.pvs?.find(p => p.name === pvName);
          if (pv?.metadata?.EGU) {
            return pv.metadata.EGU;
          }
        }
      }
    }
    return '';
  };
  
  const [properties, setProperties] = createSignal<AxisConfig>(
    props.axis || {
      id: `axis_${Date.now()}`,
      EGU: getInitialEGU(),
      position: 'left',
      autoRange: true,
      range: { low: 0, high: 100 },
      pvs: new Set(),
    }
  );

  const [useFullRange, setUseFullRange] = createSignal(false);
  const [lowInput, setLowInput] = createSignal('');
  const [highInput, setHighInput] = createSignal('');

  createEffect(() => {
    if (props.isOpen && props.axis) {
      const initialEGU = getInitialEGU();
      setProperties({
        ...props.axis,
        EGU: initialEGU || props.axis.EGU,
        range: props.axis.range || { low: 0, high: 100 },
        autoRange: props.axis.autoRange ?? true
      });
      setLowInput(props.axis.range?.low.toString() || '0');
      setHighInput(props.axis.range?.high.toString() || '100');
    }
  });


  const metaRange = () => {
    // Find first PV that has LOPR and HOPR in its metadata
    const axis = props.existingAxes.get(properties().id);
    if (!axis?.pvs.size) return null;
  
    for (const pvName of axis.pvs) {
      const pv = props.pvs?.find(p => p.name === pvName);
      if (pv?.metadata?.LOPR !== undefined && pv?.metadata?.HOPR !== undefined) {
        return {
          low: pv.metadata.LOPR,  // Already a number, no need for parseFloat
          high: pv.metadata.HOPR
        };
      }
    }
    
    return null;
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

  // Handler for toggling autoRange
  const handleAutoRangeToggle = (checked: boolean) => {
    if (checked) {
      updateProperty('autoRange', true);
    } else {
      // Use current auto range when disabling, fall back to current range if not available
      const displayRange = props.currentAutoRange || getDisplayRange();
      updateProperty('autoRange', false);
      updateProperty('range', displayRange);
      setLowInput(displayRange.low.toString());
      setHighInput(displayRange.high.toString());
    }
  };

  const getDisplayRange = () => {
    // If using full range and meta range is available, use that
    if (useFullRange() && metaRange()) {
      return metaRange()!;
    }
    // Otherwise return current range
    return properties().range || { low: 0, high: 100 };
  };

  const handleFullRangeToggle = (checked: boolean) => {
    setUseFullRange(checked);
    if (checked && metaRange()) {
      const range = metaRange()!;
      updateProperty('range', range);
      setLowInput(range.low.toString());
      setHighInput(range.high.toString());
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
              value={properties().EGU}
              onInput={(e) => updateProperty('EGU', e.currentTarget.value)}
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
              onChange={(e) => handleAutoRangeToggle(e.currentTarget.checked)}
              class="rounded border-gray-300"
            />
            <label for="autoRange" class="text-sm font-medium text-gray-700">
              Auto Range
            </label>
          </div>

          {!properties().autoRange && (
            <div class="space-y-4">
              <Show when={metaRange()}>
                <div class="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="useFullRange"
                    checked={useFullRange()}
                    onChange={(e) => handleFullRangeToggle(e.currentTarget.checked)}
                    class="rounded border-gray-300"
                  />
                  <label for="useFullRange" class="text-sm font-medium text-gray-700">
                    Use Full Range ({metaRange()?.low} to {metaRange()?.high})
                  </label>
                </div>
              </Show>

              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                  <label class="block text-sm font-medium text-gray-700">
                    Min Value
                  </label>
                  <input
                    type="text"
                    inputmode="decimal"
                    value={lowInput()}
                    onInput={(e) => {
                      setLowInput(e.currentTarget.value);
                      handleRangeChange('low', e.currentTarget.value);
                    }}
                    class="w-full px-3 py-2 border rounded-md"
                  />
                </div>
                <div class="space-y-2">
                  <label class="block text-sm font-medium text-gray-700">
                    Max Value
                  </label>
                  <input
                    type="text"
                    inputmode="decimal"
                    value={highInput()}
                    onInput={(e) => {
                      setHighInput(e.currentTarget.value);
                      handleRangeChange('high', e.currentTarget.value);
                    }}
                    class="w-full px-3 py-2 border rounded-md"
                  />
                </div>
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