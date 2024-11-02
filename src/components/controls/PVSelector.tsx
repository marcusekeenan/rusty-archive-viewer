// PVSelector.tsx
import { createSignal, For } from "solid-js";
import type { PVWithProperties, PenProperties } from "./types";
import { DEFAULT_PEN_PROPERTIES } from "./types";
import { getNextColor } from "./colors";
import PenPropertiesDialog from "./PenPropertiesDialog";
import PVListItem from "./PVListItem";

type PVSelectorProps = {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  onAddPV: (pv: string, properties: PenProperties) => void;
  onUpdatePV: (pv: string, properties: PenProperties) => void;
  onRemovePV: (pv: string) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
};

export default function PVSelector(props: PVSelectorProps) {
  const defaultPV = "ROOM:LI30:1:OUTSIDE_TEMP";
  const [searchText, setSearchText] = createSignal("");
  const [editingPV, setEditingPV] = createSignal<string | null>(null);

  // Initialize with default PV if none exists
  if (props.selectedPVs().length === 0) {
    props.onAddPV(defaultPV, DEFAULT_PEN_PROPERTIES);
  }

  const handleSearch = (e: Event) => {
    e.preventDefault();
    if (searchText().trim()) {
      // Get next available color for the new PV
      const newProperties: PenProperties = {
        ...DEFAULT_PEN_PROPERTIES,
        color: getNextColor(props.selectedPVs())
      };
      
      props.onAddPV(searchText().trim(), newProperties);
      setSearchText("");
    }
  };

  return (
    <div class="flex flex-col gap-4">
      {/* Search Form */}
      <form onSubmit={handleSearch} class="flex gap-2">
        <input
          type="text"
          value={searchText()}
          onInput={(e) => setSearchText((e.target as HTMLInputElement).value)}
          placeholder="Enter PV name"
          class="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          Add PV
        </button>
      </form>

      {/* PV List Section */}
      <div class="flex flex-col gap-2">
        <h3 class="font-medium">Selected PVs:</h3>
        {props.selectedPVs().length === 0 ? (
          <p class="text-gray-500">No PVs selected</p>
        ) : (
          <ul class="space-y-1">
            <For each={props.selectedPVs()}>
              {(pv) => (
                <PVListItem
                  pv={pv}
                  isVisible={props.visiblePVs().has(pv.name)}
                  onEdit={() => setEditingPV(pv.name)}
                  onRemove={() => props.onRemovePV(pv.name)}
                  onToggleVisibility={(isVisible) => props.onVisibilityChange(pv.name, isVisible)}
                />
              )}
            </For>
          </ul>
        )}
      </div>

      {/* Edit Dialog */}
      {editingPV() && (
        <PenPropertiesDialog
          isOpen={true}
          onClose={() => setEditingPV(null)}
          pv={editingPV()!}
          properties={
            props.selectedPVs().find((pv) => pv.name === editingPV())?.pen ||
            DEFAULT_PEN_PROPERTIES
          }
          onSave={(properties) => {
            props.onUpdatePV(editingPV()!, properties);
            setEditingPV(null);
          }}
        />
      )}
    </div>
  );
}