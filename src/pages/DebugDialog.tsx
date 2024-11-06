// DebugDialog.tsx
import { Component } from "solid-js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import type { DebugDialogProps } from "./types";

export const DebugDialog: Component<DebugDialogProps> = (props) => (
  <Dialog
    open={props.isOpen}
    onOpenChange={(isOpen) => !isOpen && props.onClose()}
  >
    <DialogContent class="max-w-4xl max-h-[80vh]">
      <DialogHeader>
        <DialogTitle>Debug Information</DialogTitle>
      </DialogHeader>
      <div class="p-4 bg-gray-50 rounded">
        <div class="overflow-auto max-h-[60vh]">
          <pre class="whitespace-pre-wrap break-words">
            {JSON.stringify(props.data, null, 2)}
          </pre>
        </div>
      </div>
    </DialogContent>
  </Dialog>
);