import { For, Show, createSignal, createEffect } from "solid-js";
import { TbPencil, TbTrash, TbChevronRight, TbPlus } from "solid-icons/tb";
import type { AxisConfig } from "../chart/types";
import type { PVWithProperties, PenProperties } from "./types";
import { AxisPropertiesDialog } from "./AxisPropertiesDialog";
import PenPropertiesDialog from "./PenPropertiesDialog";
import { DEFAULT_PEN_PROPERTIES } from "./types";
import { getNextColor } from "./colors";

interface UnifiedManagerProps {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  axes: () => Map<string, AxisConfig>;
  onAxisEdit: (axis: AxisConfig) => void;
  onAxisAdd: (axis: AxisConfig) => void;
  onAxisRemove: (axisId: string) => void;
  onAddPV: (pv: string, properties: PenProperties) => void | Promise<void>;
  onUpdatePV: (pv: string, properties: PenProperties, axisId: string) => void;
  onRemovePV: (pv: string) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
}

export default function UnifiedManager(props: UnifiedManagerProps) {
  const [expandedAxes, setExpandedAxes] = createSignal<Set<string>>(
    new Set(Array.from(props.axes().keys()))
  );
  const [editingAxis, setEditingAxis] = createSignal<AxisConfig | undefined>();
  const [isNewAxisDialogOpen, setIsNewAxisDialogOpen] = createSignal(false);
  const [editingPV, setEditingPV] = createSignal<string | null>(null);
  const [searchText, setSearchText] = createSignal("");
  const [inputMode, setInputMode] = createSignal<"single" | "multi">("single");

  createEffect(() => {
    const axisIds = Array.from(props.axes().keys());
    setExpandedAxes((prev) => {
      const newSet = new Set(prev);
      axisIds.forEach((id) => newSet.add(id));
      return newSet;
    });
  });

  const pvsByAxis = () => {
    const grouped = new Map<string, PVWithProperties[]>();
    const unassigned: PVWithProperties[] = [];

    props.selectedPVs().forEach((pv) => {
      if (pv.axisId) {
        const pvs = grouped.get(pv.axisId) || [];
        pvs.push(pv);
        grouped.set(pv.axisId, pvs);
      } else {
        unassigned.push(pv);
      }
    });

    return { grouped, unassigned };
  };

  const parsePVs = (input: string): string[] => {
    return input
      .split(/[,\n]/)
      .map((pv) => pv.trim())
      .filter((pv) => pv.length > 0);
  };

  const handleSearch = (e: Event) => {
    e.preventDefault();
    const input = searchText().trim();
    if (!input) return;

    const pvNames = parsePVs(input);
    const existingPVs = new Set(props.selectedPVs().map((pv) => pv.name));

    pvNames.forEach((pvName) => {
      if (!existingPVs.has(pvName)) {
        const newProperties: PenProperties = {
          ...DEFAULT_PEN_PROPERTIES,
          color: getNextColor([
            ...props.selectedPVs(),
            { name: pvName, pen: DEFAULT_PEN_PROPERTIES },
          ]),
        };
        props.onAddPV(pvName, newProperties);
      }
    });

    setSearchText("");
  };

  return (
    <div class="bg-white rounded-lg shadow-sm p-4">
      <form onSubmit={handleSearch} class="mb-6">
        <div class="flex justify-between items-center mb-2">
          <label class="text-sm font-medium text-gray-700">
            Add Process Variables
          </label>
          <button
            type="button"
            onClick={() =>
              setInputMode((prev) => (prev === "single" ? "multi" : "single"))
            }
            class="text-xs text-blue-500 hover:text-blue-600"
          >
            {inputMode() === "single"
              ? "Switch to Multi-line"
              : "Switch to Single-line"}
          </button>
        </div>

        {inputMode() === "single" ? (
          <div class="flex gap-2">
            <input
              type="text"
              value={searchText()}
              onInput={(e) => setSearchText(e.currentTarget.value)}
              placeholder="Enter PV names (comma-separated)"
              class="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              type="submit"
              class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              Add
            </button>
          </div>
        ) : (
          <div class="flex flex-col gap-2">
            <textarea
              value={searchText()}
              onInput={(e) => setSearchText(e.currentTarget.value)}
              placeholder="Enter PV names (one per line)"
              rows={3}
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
      </form>

      <div class="space-y-4">
        <div class="flex justify-between items-center">
          <h3 class="text-lg font-semibold">Axes & Process Variables</h3>
          <button
            type="button"
            onClick={() => setIsNewAxisDialogOpen(true)}
            class="inline-flex items-center gap-1 text-sm px-3 py-1.5 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            <TbPlus class="w-4 h-4" />
            <span>New Axis</span>
          </button>
        </div>

        <div class="space-y-2 max-h-[calc(100vh-400px)] overflow-y-auto">
          <For each={Array.from(props.axes().entries())}>
            {([axisId, axis]) => {
              const axisPVs = pvsByAxis().grouped.get(axisId) || [];
              return (
                <div class="border rounded-lg overflow-hidden">
                  <div class="bg-gray-50 p-3 flex items-center justify-between sticky top-0 z-10">
                    <div class="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => {
                          setExpandedAxes((prev) => {
                            const newSet = new Set(prev);
                            if (newSet.has(axisId)) {
                              newSet.delete(axisId);
                            } else {
                              newSet.add(axisId);
                            }
                            return newSet;
                          });
                        }}
                        class="flex items-center gap-2"
                      >
                        <TbChevronRight
                          class={`w-4 h-4 transition-transform ${
                            expandedAxes().has(axisId) ? "rotate-90" : ""
                          }`}
                        />
                        <span class="font-medium">{axis.EGU}</span>
                      </button>
                      <span class="text-xs px-1.5 py-0.5 bg-gray-200 rounded">
                        {axis.position}
                      </span>
                      <span class="text-xs text-gray-500">
                        {axisPVs.length} PV{axisPVs.length !== 1 ? "s" : ""}
                      </span>
                    </div>

                    <div class="flex gap-2">
                      <button
                        type="button"
                        onClick={() => setEditingAxis(axis)}
                        class="p-1 text-blue-500 hover:text-blue-700 rounded"
                        title="Edit Axis"
                      >
                        <TbPencil class="w-4 h-4" />
                      </button>
                      {axisPVs.length === 0 && (
                        <button
                          type="button"
                          onClick={() => props.onAxisRemove(axisId)}
                          class="p-1 text-red-500 hover:text-red-700 rounded"
                          title="Remove Axis"
                        >
                          <TbTrash class="w-4 h-4" />
                        </button>
                      )}
                    </div>
                  </div>

                  <Show when={expandedAxes().has(axisId)}>
                    <div class="border-t max-h-[200px] overflow-y-auto">
                      {axisPVs.length > 0 ? (
                        <ul class="divide-y">
                          <For each={axisPVs}>
                            {(pv) => (
                              <li class="p-2 hover:bg-gray-50">
                                <div class="flex items-center gap-2">
                                  <input
                                    type="checkbox"
                                    checked={props.visiblePVs().has(pv.name)}
                                    onChange={(e) =>
                                      props.onVisibilityChange(
                                        pv.name,
                                        e.currentTarget.checked
                                      )
                                    }
                                    class="w-4 h-4 cursor-pointer"
                                    style={{
                                      "accent-color": pv.pen.color,
                                    }}
                                  />
                                  <span
                                    class={`flex-grow truncate ${props.visiblePVs().has(pv.name) ? "" : "text-gray-400"}`}
                                    title={pv.name}
                                  >
                                    {pv.name}
                                  </span>
                                  <button
                                    onClick={() => setEditingPV(pv.name)}
                                    class="p-1 text-blue-500 hover:text-blue-700 rounded"
                                    title="Edit PV"
                                  >
                                    <TbPencil class="w-4 h-4" />
                                  </button>
                                  <button
                                    onClick={() => props.onRemovePV(pv.name)}
                                    class="p-1 text-red-500 hover:text-red-700 rounded"
                                    title="Remove PV"
                                  >
                                    <TbTrash class="w-4 h-4" />
                                  </button>
                                </div>
                              </li>
                            )}
                          </For>
                        </ul>
                      ) : (
                        <div class="p-4 text-center text-gray-500">
                          No PVs assigned to this axis
                        </div>
                      )}
                    </div>
                  </Show>
                </div>
              );
            }}
          </For>

          <Show when={pvsByAxis().unassigned.length > 0}>
            <div class="border rounded-lg">
              <div class="bg-gray-50 p-3 sticky top-0 z-10">
                <h4 class="font-medium text-gray-700">Unassigned PVs</h4>
              </div>
              <div class="max-h-[200px] overflow-y-auto">
                <ul class="divide-y">
                  <For each={pvsByAxis().unassigned}>
                    {(pv) => (
                      <li class="p-2 hover:bg-gray-50">
                        <div class="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={props.visiblePVs().has(pv.name)}
                            onChange={(e) =>
                              props.onVisibilityChange(
                                pv.name,
                                e.currentTarget.checked
                              )
                            }
                            class="w-4 h-4 cursor-pointer"
                            style={{
                              "accent-color": pv.pen.color,
                            }}
                          />
                          <span
                            class={`flex-grow truncate ${props.visiblePVs().has(pv.name) ? "" : "text-gray-400"}`}
                            title={pv.name}
                          >
                            {pv.name}
                          </span>
                          <button
                            onClick={() => setEditingPV(pv.name)}
                            class="p-1 text-blue-500 hover:text-blue-700 rounded"
                            title="Edit PV"
                          >
                            <TbPencil class="w-4 h-4" />
                          </button>
                          <button
                            onClick={() => props.onRemovePV(pv.name)}
                            class="p-1 text-red-500 hover:text-red-700 rounded"
                            title="Remove PV"
                          >
                            <TbTrash class="w-4 h-4" />
                          </button>
                        </div>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </div>
          </Show>
        </div>
      </div>

      {/* Dialogs */}
      <Show when={editingAxis()}>
  <AxisPropertiesDialog
    isOpen={true}
    onClose={() => setEditingAxis(undefined)}
    axis={editingAxis()}
    existingAxes={props.axes()}
    onSave={(updatedAxis) => {
      props.onAxisEdit(updatedAxis);
      // Note: Don't close the dialog here
    }}
  />
</Show>

<Show when={isNewAxisDialogOpen()}>
  <AxisPropertiesDialog
    isOpen={true}
    onClose={() => setIsNewAxisDialogOpen(false)}
    existingAxes={props.axes()}
    onSave={(newAxis) => {
      props.onAxisAdd(newAxis);
      // Note: Don't close the dialog here
    }}
  />
</Show>

      <Show when={editingPV()}>
        <PenPropertiesDialog
          isOpen={true}
          onClose={() => setEditingPV(null)}
          pv={editingPV()!}
          properties={
            props.selectedPVs().find((p) => p.name === editingPV())?.pen ||
            DEFAULT_PEN_PROPERTIES
          }
          availableAxes={props.axes()}
          selectedAxisId={
            props.selectedPVs().find((p) => p.name === editingPV())?.axisId
          }
          onSave={(properties, axisId) => {
            // First update the PV
            props.onUpdatePV(editingPV()!, properties, axisId);
            // Then close the dialog after a short delay
            setTimeout(() => setEditingPV(null), 100);
          }}
        />
      </Show>
    </div>
  );
}
