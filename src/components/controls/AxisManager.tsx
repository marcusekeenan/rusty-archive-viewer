import { For, createSignal } from "solid-js";
import { AxisConfig } from "../chart/types";
import { AxisPropertiesDialog } from "../AxisPropertiesDialog";

interface AxisManagerProps {
  axes: () => Map<string, AxisConfig>;
  onAxisEdit: (axis: AxisConfig) => void;
  onAxisAdd: (axis: AxisConfig) => void;
  onAxisRemove: (axisId: string) => void;
}

export default function AxisManager(props: AxisManagerProps) {
    const [editingAxis, setEditingAxis] = createSignal<AxisConfig | undefined>();
    const [isNewAxisDialogOpen, setIsNewAxisDialogOpen] = createSignal(false);
    const [expandedAxis, setExpandedAxis] = createSignal<string | null>(null);
  
    const handleRemoveAxis = (e: Event, axisId: string) => {
      e.stopPropagation();
      props.onAxisRemove(axisId);
    };
  
    const handleEditAxis = (e: Event, axis: AxisConfig) => {
      e.stopPropagation();
      setEditingAxis(axis);
    };
  
    return (
      <div class="bg-white rounded-lg shadow-sm p-4">
        <h3 class="text-lg font-semibold mb-2">Y-Axes</h3>
        
        {/* Axis List */}
        <div class="space-y-2 mb-4">
          <For each={Array.from(props.axes().values())}>
            {(axis) => (
              <div class="p-3 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors">
                <div class="flex justify-between items-center">
                  <button
                    type="button"
                    onClick={() => setExpandedAxis(
                      expandedAxis() === axis.id ? null : axis.id
                    )}
                    class="flex items-center gap-2 text-left flex-1"
                  >
                    <span class="font-medium">{axis.egu}</span>
                    <span class="text-xs px-1.5 py-0.5 bg-gray-200 rounded">
                      {axis.position}
                    </span>
                    <span class="text-xs text-gray-500">
                      {axis.pvs.size} PV{axis.pvs.size !== 1 ? 's' : ''}
                    </span>
                  </button>
  
                  <div class="flex gap-2">
                    <button 
                      type="button"
                      onClick={(e) => handleEditAxis(e, axis)}
                      class="text-blue-500 hover:text-blue-700 px-2 py-1"
                    >
                      Edit
                    </button>
                    {axis.pvs.size === 0 && (
                      <button
                        type="button"
                        onClick={(e) => handleRemoveAxis(e, axis.id)}
                        class="text-red-500 hover:text-red-700 px-2 py-1"
                      >
                        Remove
                      </button>
                    )}
                  </div>
                </div>
  
                {expandedAxis() === axis.id && (
                  <div class="mt-2 pl-4 border-t pt-2">
                    <div class="text-sm space-y-1">
                      <div>
                        {axis.autoRange ? 'Auto Range' : `Range: ${axis.range?.low} to ${axis.range?.high}`}
                      </div>
                      {axis.pvs.size > 0 && (
                        <div>
                          <div class="text-gray-500 mb-1">Assigned PVs:</div>
                          <div class="pl-2">
                            <For each={Array.from(axis.pvs)}>
                              {(pv) => <div>{pv}</div>}
                            </For>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            )}
          </For>
        </div>
  
        <button 
          type="button"
          onClick={() => setIsNewAxisDialogOpen(true)}
          class="w-full py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          Add New Axis
        </button>
  
        <AxisPropertiesDialog
          isOpen={!!editingAxis()}
          onClose={() => setEditingAxis(undefined)}
          axis={editingAxis()}
          existingAxes={props.axes()}
          onSave={(updatedAxis) => {
            props.onAxisEdit(updatedAxis);
            setEditingAxis(undefined);
          }}
        />
  
        <AxisPropertiesDialog
          isOpen={isNewAxisDialogOpen()}
          onClose={() => setIsNewAxisDialogOpen(false)}
          existingAxes={props.axes()}
          onSave={(newAxis) => {
            props.onAxisAdd(newAxis);
            setIsNewAxisDialogOpen(false);
          }}
        />
      </div>
    );
  }