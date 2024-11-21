import { createSignal, createEffect } from 'solid-js';
import type { PenProperties } from './types';

interface PenPropertiesDialogProps {
  isOpen: boolean;
  onClose: () => void;
  pv: string;
  properties: PenProperties;
  onSave: (properties: PenProperties) => void;
}

const debounce = <T extends (...args: any[]) => any>(
  fn: T,
  delay: number
) => {
  let timeoutId: number;
  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = window.setTimeout(() => fn(...args), delay);
  };
};

export default function PenPropertiesDialog(props: PenPropertiesDialogProps) {
  const [properties, setProperties] = createSignal(props.properties);
  
  const debouncedSave = debounce((newProps: PenProperties) => {
    props.onSave(newProps);
  }, 150);

  createEffect(() => {
    if (props.isOpen) {
      setProperties(props.properties);
    }
  });

  const updateProperty = <K extends keyof PenProperties>(
    key: K,
    value: PenProperties[K]
  ) => {
    const newProps = { ...properties(), [key]: value };
    setProperties(newProps);
    debouncedSave(newProps);
  };

  if (!props.isOpen) return null;

  return (
    <div class="fixed inset-0 z-50">
      {/* Backdrop */}
      <div 
        class="fixed inset-0 bg-black/50" 
        onClick={props.onClose}
      />

      {/* Modal */}
      <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] bg-white rounded-lg shadow-lg">
        {/* Header */}
        <div class="flex items-center justify-between p-4 border-b">
          <h2 class="text-lg font-semibold">Line Properties: {props.pv}</h2>
          <button 
            onClick={props.onClose}
            class="text-gray-500 hover:text-gray-700 text-2xl leading-none"
          >
            ×
          </button>
        </div>

        {/* Content */}
        <div class="p-4 space-y-6">
          {/* Color */}
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">Color</label>
            <div class="flex items-center gap-3">
              <input
                type="color"
                value={properties().color}
                onChange={(e) => updateProperty('color', e.currentTarget.value)}
                class="w-10 h-10 rounded cursor-pointer"
              />
              <span class="text-sm text-gray-500">{properties().color}</span>
            </div>
          </div>

          {/* Line Width */}
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Line Width: {properties().lineWidth}px
            </label>
            <div class="space-y-1">
              <input
                type="range"
                min="1"
                max="5"
                step="0.5"
                value={properties().lineWidth}
                onChange={(e) => updateProperty('lineWidth', parseFloat(e.currentTarget.value))}
                class="w-full"
              />
              <div class="h-[3px] rounded-full" style={{
                "background-color": properties().color,
                "height": `${properties().lineWidth}px`,
                "opacity": properties().opacity
              }}/>
            </div>
          </div>

          {/* Line Style */}
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">Style</label>
            <select
              value={properties().style}
              onChange={(e) => updateProperty('style', e.currentTarget.value as PenProperties['style'])}
              class="w-full px-3 py-2 border rounded-md"
            >
              <option value="solid">Solid</option>
              <option value="dashed">Dashed</option>
              <option value="dotted">Dotted</option>
            </select>
          </div>

          {/* Opacity */}
          <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
              Opacity: {Math.round(properties().opacity * 100)}%
            </label>
            <div class="space-y-1">
              <input
                type="range"
                min="0"
                max="1"
                step="0.1"
                value={properties().opacity}
                onChange={(e) => updateProperty('opacity', parseFloat(e.currentTarget.value))}
                class="w-full"
              />
              <div class="h-2 rounded-full" style={{
                "background-color": properties().color,
                "opacity": properties().opacity
              }}/>
            </div>
          </div>

          {/* Show Points */}
          <div class="space-y-2">
            <div class="flex items-center gap-2">
              <input
                type="checkbox"
                id="show-points"
                checked={properties().showPoints}
                onChange={(e) => updateProperty('showPoints', e.currentTarget.checked)}
                class="rounded border-gray-300"
              />
              <label for="show-points" class="text-sm font-medium text-gray-700">
                Show Points
              </label>
            </div>

            {properties().showPoints && (
              <div class="pl-6 space-y-1">
                <label class="block text-sm text-gray-700">
                  Point Size: {properties().pointSize}px
                </label>
                <input
                  type="range"
                  min="2"
                  max="8"
                  value={properties().pointSize}
                  onChange={(e) => updateProperty('pointSize', parseInt(e.currentTarget.value))}
                  class="w-full"
                />
              </div>
            )}
          </div>

          {/* Preview */}
          <div class="mt-6 p-4 border rounded-lg bg-gray-50">
            <div class="h-12 flex items-center justify-center relative">
              <div class="absolute inset-x-4 top-1/2" style={{
                "border-top-style": properties().style,
                "border-top-width": `${properties().lineWidth}px`,
                "border-top-color": properties().color,
                "opacity": properties().opacity
              }}>
                {properties().showPoints && (
                  <div class="flex justify-between absolute inset-x-0">
                    {[0, 1, 2].map((_, i) => (
                      <div
                        style={{
                          "width": `${properties().pointSize}px`,
                          "height": `${properties().pointSize}px`,
                          "background-color": properties().color,
                          "opacity": properties().opacity,
                          "border-radius": "50%",
                          "transform": "translateY(-50%)"
                        }}
                      />
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}