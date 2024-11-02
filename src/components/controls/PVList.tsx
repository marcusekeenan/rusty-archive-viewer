// PVList.tsx
import { For } from "solid-js";
import type { PVWithProperties } from "./types";
import PVListItem from "./PVListItem";

type PVListProps = {
  pvs: PVWithProperties[];
  visiblePVs: () => Set<string>;
  onEditPV: (pv: PVWithProperties) => void;
  onRemovePV: (pvName: string) => void;
  onVisibilityChange: (pvName: string, isVisible: boolean) => void;
};

export default function PVList(props: PVListProps) {
  return (
    <ul class="space-y-1">
      <For each={props.pvs}>
        {(pv) => (
          <PVListItem
            pv={pv}
            isVisible={props.visiblePVs().has(pv.name)}
            onEdit={() => props.onEditPV(pv)}
            onRemove={() => props.onRemovePV(pv.name)}
            onToggleVisibility={(isVisible) => props.onVisibilityChange(pv.name, isVisible)}
          />
        )}
      </For>
    </ul>
  );
}
