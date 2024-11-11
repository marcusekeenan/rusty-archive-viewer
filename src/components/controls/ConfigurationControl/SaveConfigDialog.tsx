import { Component } from 'solid-js';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../ui/dialog";
import type { SaveConfigDialogProps } from '../types';

/**
 * Dialog for saving configurations
 */
export const SaveConfigDialog: Component<SaveConfigDialogProps> = (props) => {
  return (
    <Dialog>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>SaveConfigDialog</DialogTitle>
        </DialogHeader>
        {/* Dialog implementation */}
      </DialogContent>
    </Dialog>
  );
};

export default SaveConfigDialog;
