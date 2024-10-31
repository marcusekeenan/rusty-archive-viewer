// Dialog.tsx

import { Dialog as DialogPrimitive } from "@kobalte/core";
import type { ParentProps } from "solid-js";

const Dialog = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogPortal = DialogPrimitive.Portal;
const DialogOverlay = DialogPrimitive.Overlay;
const DialogContent = DialogPrimitive.Content;
const DialogTitle = DialogPrimitive.Title;
const DialogDescription = DialogPrimitive.Description;

// Define DialogHeader component
const DialogHeader = (props: ParentProps) => {
  return <div class="dialog-header">{props.children}</div>;
};

// Define DialogFooter component
const DialogFooter = (props: ParentProps) => {
  return <div class="dialog-footer">{props.children}</div>;
};

export {
  Dialog,
  DialogTrigger,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
};
