import { createSignal } from "solid-js";
import type { PVWithProperties, PenProperties } from "../../types";
import { DEFAULT_PEN_PROPERTIES } from "../../types";
import PenPropertiesDialog from "../dialogs/PenPropertiesDialog";

type PVSelectorProps = {
  selectedPVs: () => PVWithProperties[];
  onAddPV: (pv: string, properties: PenProperties) => void;
  onUpdatePV: (pv: string, properties: PenProperties) => void;
  onRemovePV: (pv: string) => void;
};

export default function PVSelector(props: PVSelectorProps) {
  const defaultPV = "ROOM:LI30:1:OUTSIDE_TEMP";
  if (props.selectedPVs().length === 0) {
    props.onAddPV(defaultPV, DEFAULT_PEN_PROPERTIES);
  }

  const [searchText, setSearchText] = createSignal("");
  const [editingPV, setEditingPV] = createSignal<string | null>(null);

  const handleSearch = (e: Event) => {
    e.preventDefault();
    if (searchText().trim()) {
      props.onAddPV(searchText().trim(), DEFAULT_PEN_PROPERTIES);
      setSearchText("");
    }
  };

  const handleEditProperties = (pv: PVWithProperties) => {
    setEditingPV(pv.name);
  };

  return (
    <div class="flex flex-col gap-4">
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

      <div class="flex flex-col gap-2">
        <h3 class="font-medium">Selected PVs:</h3>
        {props.selectedPVs().length === 0 ? (
          <p class="text-gray-500">No PVs selected</p>
        ) : (
          <ul class="space-y-2">
            {props.selectedPVs().map((pv) => (
              <li
                class="group flex items-center p-2 bg-gray-50 rounded hover:bg-gray-100 cursor-pointer transition-colors"
                onClick={() => handleEditProperties(pv)}
              >
                <div
                  class="w-3 h-3 rounded-full mr-3 border border-gray-300"
                  style={{
                    "background-color": pv.pen.color,
                    opacity: pv.pen.opacity,
                  }}
                />
                <span class="flex-grow truncate">{pv.name}</span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onRemovePV(pv.name);
                  }}
                  class="opacity-0 group-hover:opacity-100 transition-opacity p-1 text-gray-400 hover:text-red-500"
                  title="Remove PV"
                >
                  {/* Using a simple × instead of an icon */}
                  <span class="block w-4 h-4 text-center leading-4">×</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

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