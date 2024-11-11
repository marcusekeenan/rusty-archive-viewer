import { createSignal } from 'solid-js';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../ui/dialog";
import type { PenProperties } from '../types';

type PenPropertiesDialogProps = {
  isOpen: boolean;
  onClose: () => void;
  pv: string;
  properties: PenProperties;
  onSave: (properties: PenProperties) => void;
};

// Change to default export
export default function PenPropertiesDialog(props: PenPropertiesDialogProps) {
  const [properties, setProperties] = createSignal(props.properties);

  const handleSave = () => {
    props.onSave(properties());
    props.onClose();
  };

  return (
    <Dialog open={props.isOpen} onOpenChange={(open) => !open && props.onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit Line Properties for {props.pv}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium mb-1">Color</label>
            <input
              type="color"
              value={properties().color}
              onChange={(e) => setProperties(p => ({ ...p, color: e.target.value }))}
              class="w-full"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-1">Line Width</label>
            <input
              type="range"
              min="1"
              max="5"
              step="0.5"
              value={properties().lineWidth}
              onChange={(e) => setProperties(p => ({ ...p, width: parseFloat(e.target.value) }))}
              class="w-full"
            />
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Line Style</label>
            <select
              value={properties().style}
              onChange={(e) => setProperties(p => ({ 
                ...p, 
                style: e.target.value as PenProperties['style'] 
              }))}
              class="w-full px-3 py-2 border rounded"
            >
              <option value="solid">Solid</option>
              <option value="dashed">Dashed</option>
              <option value="dotted">Dotted</option>
            </select>
          </div>

          <div>
            <label class="flex items-center">
              <input
                type="checkbox"
                checked={properties().showPoints}
                onChange={(e) => setProperties(p => ({ ...p, showPoints: e.target.checked }))}
                class="mr-2"
              />
              Show Points
            </label>
          </div>

          {properties().showPoints && (
            <div>
              <label class="block text-sm font-medium mb-1">Point Size</label>
              <input
                type="range"
                min="2"
                max="8"
                value={properties().pointSize}
                onChange={(e) => setProperties(p => ({ ...p, pointSize: parseInt(e.target.value) }))}
                class="w-full"
              />
            </div>
          )}

          <div>
            <label class="block text-sm font-medium mb-1">Opacity</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={properties().opacity}
              onChange={(e) => setProperties(p => ({ ...p, opacity: parseFloat(e.target.value) }))}
              class="w-full"
            />
          </div>

          <div class="flex justify-end gap-2 mt-4">
            <button
              onClick={props.onClose}
              class="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              Save
            </button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}