import { For, createSignal } from "solid-js";
import { AxisConfig, AxisAssignment } from "../chart/types";
import { AxisPropertiesDialog } from "../AxisPropertiesDialog";

interface AxisManagerProps {
  axes: () => Map<string, AxisConfig>;
  onAxisChange: (assignment: AxisAssignment) => void;
  onAxisEdit: (axis: AxisConfig) => void;
  onAxisAdd: (axis: AxisConfig) => void;
}

export default function AxisManager(props: AxisManagerProps) {
  const [editingAxis, setEditingAxis] = createSignal<AxisConfig | undefined>();
  const [isNewAxisDialogOpen, setIsNewAxisDialogOpen] = createSignal(false);
  const [expandedAxis, setExpandedAxis] = createSignal<string | null>(null);

  const handleAxisEdit = (axis: AxisConfig) => {
    setEditingAxis(axis);
  };

  const handleAxisSave = (updatedAxis: AxisConfig) => {
    props.onAxisEdit(updatedAxis);
    setEditingAxis(undefined);
  };

  const handleNewAxis = () => {
    setIsNewAxisDialogOpen(true);
  };

  const handleNewAxisSave = (axis: AxisConfig) => {
    props.onAxisAdd(axis);
    setIsNewAxisDialogOpen(false);
  };

  const handlePVAxisChange = (pvName: string, fromAxisId: string, toAxisId: string) => {
    props.onAxisChange({
      pvName,
      axisId: toAxisId,
      autoRange: true
    });
  };

  return (
    <div class="bg-white rounded-lg shadow-sm p-4">
      <h3 class="text-lg font-semibold mb-2">Y-Axes</h3>
      <For each={Array.from(props.axes().values())}>
        {(axis) => (
          <div class="mb-2 p-2 bg-gray-100 rounded">
            <div class="flex justify-between items-center">
              <button
                onClick={() => setExpandedAxis(expandedAxis() === axis.id ? null : axis.id)}
                class="flex items-center gap-2"
              >
                <span>{axis.egu} ({axis.position})</span>
                <span class="text-xs text-gray-500">
                  {Array.from(axis.pvs).length} PVs
                </span>
              </button>
              <button 
                onClick={() => handleAxisEdit(axis)}
                class="text-blue-500 hover:text-blue-700"
              >
                Edit
              </button>
            </div>

            {expandedAxis() === axis.id && (
              <div class="mt-2 pl-4 space-y-1">
                <For each={Array.from(axis.pvs)}>
                  {(pvName) => (
                    <div class="flex items-center justify-between text-sm">
                      <span>{pvName}</span>
                      <select
                        value={axis.id}
                        onChange={(e) => handlePVAxisChange(
                          pvName,
                          axis.id,
                          e.currentTarget.value
                        )}
                        class="text-sm px-2 py-1 border rounded"
                      >
                        <For each={Array.from(props.axes().values())}>
                          {(targetAxis) => (
                            <option value={targetAxis.id}>
                              {targetAxis.egu} ({targetAxis.position})
                            </option>
                          )}
                        </For>
                      </select>
                    </div>
                  )}
                </For>
              </div>
            )}
          </div>
        )}
      </For>

      <button 
        onClick={handleNewAxis}
        class="mt-2 w-full py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
      >
        Add New Axis
      </button>

      {/* Edit Axis Dialog */}
      <AxisPropertiesDialog
        isOpen={!!editingAxis()}
        onClose={() => setEditingAxis(undefined)}
        axis={editingAxis()}
        existingAxes={props.axes()}
        onSave={handleAxisSave}
      />

      {/* New Axis Dialog */}
      <AxisPropertiesDialog
        isOpen={isNewAxisDialogOpen()}
        onClose={() => setIsNewAxisDialogOpen(false)}
        existingAxes={props.axes()}
        onSave={handleNewAxisSave}
      />
    </div>
  );
}