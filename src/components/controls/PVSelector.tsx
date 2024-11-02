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
  const [inputMode, setInputMode] = createSignal<'single' | 'multi'>('single');

  // Initialize with default PV if none exists
  if (props.selectedPVs().length === 0) {
    props.onAddPV(defaultPV, DEFAULT_PEN_PROPERTIES);
  }

  const parsePVs = (input: string): string[] => {
    // Split by commas or newlines and clean up each entry
    return input
      .split(/[,\n]/)
      .map(pv => pv.trim())
      .filter(pv => pv.length > 0);
  };

  const handleSearch = (e: Event) => {
    e.preventDefault();
    const input = searchText().trim();
    if (!input) return;

    const pvNames = parsePVs(input);
    const existingPVs = new Set(props.selectedPVs().map(pv => pv.name));

    // Add each new PV with a unique color
    pvNames.forEach(pvName => {
      if (!existingPVs.has(pvName)) {
        const newProperties: PenProperties = {
          ...DEFAULT_PEN_PROPERTIES,
          color: getNextColor([...props.selectedPVs(), { name: pvName, pen: DEFAULT_PEN_PROPERTIES }])
        };
        props.onAddPV(pvName, newProperties);
      }
    });

    setSearchText("");
  };

  const toggleInputMode = () => {
    setInputMode(prev => prev === 'single' ? 'multi' : 'single');
    setSearchText("");
  };

  return (
    <div class="flex flex-col gap-4">
      {/* Search Form */}
      <form onSubmit={handleSearch} class="flex flex-col gap-2">
        <div class="flex justify-between items-center">
          <label class="text-sm text-gray-600">
            Add Process Variables
          </label>
          <button
            type="button"
            onClick={toggleInputMode}
            class="text-xs text-blue-500 hover:text-blue-600"
          >
            {inputMode() === 'single' ? 'Switch to Multi-line' : 'Switch to Single-line'}
          </button>
        </div>
        
        {inputMode() === 'single' ? (
          <div class="flex gap-2">
            <input
              type="text"
              value={searchText()}
              onInput={(e) => setSearchText((e.target as HTMLInputElement).value)}
              placeholder="Enter PV names (comma-separated)"
              class="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              type="submit"
              class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              Add PVs
            </button>
          </div>
        ) : (
          <div class="flex flex-col gap-2">
            <textarea
              value={searchText()}
              onInput={(e) => setSearchText((e.target as HTMLTextAreaElement).value)}
              placeholder="Enter PV names (one per line)"
              rows={5}
              class="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
            />
            <button
              type="submit"
              class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              Add PVs
            </button>
          </div>
        )}

        <div class="text-xs text-gray-500">
          {inputMode() === 'single' 
            ? "Separate multiple PVs with commas" 
            : "Enter each PV on a new line"}
        </div>
      </form>

      {/* PV List Section */}
      <div class="flex flex-col gap-2">
        <div class="flex justify-between items-center">
          <h3 class="font-medium">Selected PVs:</h3>
          <span class="text-sm text-gray-500">
            {props.selectedPVs().length} PVs
          </span>
        </div>
        
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