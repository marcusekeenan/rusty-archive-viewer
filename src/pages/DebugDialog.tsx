// DebugDialog.tsx
import { Component, For } from "solid-js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import type { DebugDialogProps } from "./types";

const DebugDialog: Component<DebugDialogProps> = (props) => (
  <Dialog
    open={props.isOpen}
    onOpenChange={(isOpen) => !isOpen && props.onClose()}
  >
    <DialogContent class="fixed inset-0 flex items-center justify-center z-50">
      <div class="bg-white rounded-lg shadow-lg max-w-3xl w-full max-h-[80vh] overflow-y-auto p-6">
        <DialogHeader>
          <DialogTitle>Debug Information</DialogTitle>
        </DialogHeader>
        <div class="mt-4">
          {/* Display each debug log */}
          <For each={props.data}>
            {(log) => (
              <div class="mb-4 p-2 border-b border-gray-200">
                <p><strong>Timestamp:</strong> {log.timestamp}</p>
                <p><strong>Message:</strong> {log.message}</p>
                <p><strong>Type:</strong> <span class={`text-${log.type === "error" ? "red" : log.type === "debug" ? "blue" : "green"}-500`}>{log.type}</span></p>
                {log.details && (
                  <pre class="mt-2 bg-gray-100 p-2 rounded text-sm text-gray-700 whitespace-pre-wrap">
                    {log.details}
                  </pre>
                )}
              </div>
            )}
          </For>
        </div>
        <button
          onClick={props.onClose}
          class="mt-4 bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600"
        >
          Close
        </button>
      </div>
    </DialogContent>
  </Dialog>
);

export default DebugDialog;
