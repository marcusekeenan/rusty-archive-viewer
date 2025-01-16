// import { For, createSignal, Show } from "solid-js";
// import { AxisConfig } from "../../types";
// import { AxisPropertiesDialog } from "./AxisPropertiesDialog";

// interface AxisManagerProps {
//   axes: () => Map<string, AxisConfig>;
//   onAxisEdit: (axis: AxisConfig) => void;
//   onAxisAdd: (axis: AxisConfig) => void;
//   onAxisRemove: (axisId: string) => void;
// }

// export default function AxisManager(props: AxisManagerProps) {
//     const [editingAxis, setEditingAxis] = createSignal<AxisConfig | undefined>();
//     const [isNewAxisDialogOpen, setIsNewAxisDialogOpen] = createSignal(false);
//     const [expandedAxis, setExpandedAxis] = createSignal<string | null>(null);
  
//     const handleRemoveAxis = (e: Event, axisId: string) => {
//       e.stopPropagation();
//       props.onAxisRemove(axisId);
//     };
  
//     const handleEditAxis = (e: Event, axis: AxisConfig) => {
//       e.stopPropagation();
//       console.log("Opening edit dialog for axis:", axis);
//       setEditingAxis({...axis});
//     };

//     const handleAxisUpdate = (updatedAxis: AxisConfig) => {
//       props.onAxisEdit(updatedAxis);
//     };

//     const handleNewAxisUpdate = (newAxis: AxisConfig) => {
//       props.onAxisAdd(newAxis);
//     };
  
//     return (
//       <div class="bg-white rounded-lg shadow-sm p-4">
//         <h3 class="text-lg font-semibold mb-2">Y-Axes</h3>
        
//         <div class="space-y-2 mb-4">
//           <For each={Array.from(props.axes().values())}>
//             {(axis) => (
//               <div class="p-3 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors">
//                 <div class="flex justify-between items-center">
//                   <button
//                     type="button"
//                     onClick={() => setExpandedAxis(
//                       expandedAxis() === axis.id ? null : axis.id
//                     )}
//                     class="flex items-center gap-2 text-left flex-1"
//                   >
//                     <span class="font-medium">{axis.EGU}</span>
//                     <span class="text-xs px-1.5 py-0.5 bg-gray-200 rounded">
//                       {axis.position}
//                     </span>
//                     <span class="text-xs text-gray-500">
//                       {axis.pvs.size} PV{axis.pvs.size !== 1 ? 's' : ''}
//                     </span>
//                   </button>
  
//                   <div class="flex gap-2">
//                     <button 
//                       type="button"
//                       onClick={(e) => handleEditAxis(e, axis)}
//                       class="text-blue-500 hover:text-blue-700 px-2 py-1"
//                     >
//                       Edit
//                     </button>
//                     {axis.pvs.size === 0 && (
//                       <button
//                         type="button"
//                         onClick={(e) => handleRemoveAxis(e, axis.id)}
//                         class="text-red-500 hover:text-red-700 px-2 py-1"
//                       >
//                         Remove
//                       </button>
//                     )}
//                   </div>
//                 </div>
  
//                 {expandedAxis() === axis.id && (
//                   <div class="mt-2 pl-4 border-t pt-2">
//                     <div class="text-sm space-y-1">
//                       <div>
//                         {axis.autoRange ? 'Auto Range' : `Range: ${axis.range?.low} to ${axis.range?.high}`}
//                       </div>
//                       {axis.pvs.size > 0 && (
//                         <div>
//                           <div class="text-gray-500 mb-1">Assigned PVs:</div>
//                           <div class="pl-2">
//                             <For each={Array.from(axis.pvs)}>
//                               {(pv) => <div>{pv}</div>}
//                             </For>
//                           </div>
//                         </div>
//                       )}
//                     </div>
//                   </div>
//                 )}
//               </div>
//             )}
//           </For>
//         </div>
  
//         <button 
//           type="button"
//           onClick={() => setIsNewAxisDialogOpen(true)}
//           class="w-full py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
//         >
//           Add New Axis
//         </button>
  
//         <Show when={editingAxis()}>
//           <AxisPropertiesDialog
//             isOpen={true}
//             onClose={() => setEditingAxis(undefined)}
//             axis={editingAxis()}
//             existingAxes={props.axes()}
//             pvs={props.selectedPVs()}  // Add PVs prop
//             onSave={handleAxisUpdate}
//           />
//         </Show>

//         <Show when={isNewAxisDialogOpen()}>
//           <AxisPropertiesDialog
//             isOpen={true}
//             onClose={() => setIsNewAxisDialogOpen(false)}
//             existingAxes={props.axes()}
//             pvs={props.selectedPVs()}  // Add PVs prop
//             onSave={handleNewAxisUpdate}
//           />
//         </Show>
//       </div>
//     );
//   }