// PVListItem.tsx
import type { PVWithProperties } from "../../../types";

type PVListItemProps = {
  pv: PVWithProperties;
  isVisible: boolean;
  onEdit: () => void;
  onRemove: () => void;
  onToggleVisibility: (isVisible: boolean) => void;
};

export default function PVListItem(props: PVListItemProps) {
  return (
    <li class="group flex items-center p-2 bg-gray-50 rounded hover:bg-gray-100 transition-colors text-sm">
      {/* Visibility checkbox with PV color */}
      <label class="flex items-center cursor-pointer">
        <input
          type="checkbox"
          checked={props.isVisible}
          onChange={(e) => props.onToggleVisibility(e.currentTarget.checked)}
          class="w-4 h-4 cursor-pointer"
          style={{
            "accent-color": props.pv.pen.color,
          }}
        />
      </label>

      {/* PV name - click to edit */}
      <div 
        class="flex-grow truncate cursor-pointer ml-2"
        onClick={props.onEdit}
      >
        <span class={props.isVisible ? '' : 'text-gray-400'}>
          {props.pv.name}
        </span>
      </div>

      {/* Remove button */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          props.onRemove();
        }}
        class="opacity-0 group-hover:opacity-100 transition-opacity p-1 text-gray-400 hover:text-red-500"
        title="Remove PV"
      >
        <span class="block w-4 h-4 text-center leading-4">×</span>
      </button>
    </li>
  );
}