// src/components/controls/common/hooks/usePVManagement.ts

import { createSignal, createEffect } from "solid-js";
import type { PVWithProperties, PenProperties } from "../../../../types";
import { getNextColor } from "../../../../utils/colorUtils";

export interface UsePVManagementOptions {
  initialPVs?: PVWithProperties[];
  onError?: (error: Error) => void;
  maxPVs?: number;
}

export interface UsePVManagementReturn {
  selectedPVs: () => PVWithProperties[];
  visiblePVs: () => Set<string>;
  handleAddPV: (pv: string, properties?: Partial<PenProperties>) => void;
  handleRemovePV: (pv: string) => void;
  handleUpdatePV: (pv: string, properties: Partial<PenProperties>) => void;
  handleVisibilityChange: (pvName: string, isVisible: boolean) => void;
}

export function usePVManagement(options: UsePVManagementOptions = {}): UsePVManagementReturn {
  const {
    initialPVs = [],
    onError,
    maxPVs = 100
  } = options;

  // Local state
  const [selectedPVs, setSelectedPVs] = createSignal<PVWithProperties[]>(initialPVs);
  const [visiblePVs, setVisiblePVs] = createSignal<Set<string>>(
    new Set(initialPVs.filter(pv => pv.pen?.visible !== false).map(pv => pv.name))
  );

  // Create default pen properties with next available color
  const createDefaultPen = (existingPVs: PVWithProperties[]): PenProperties => ({
    color: getNextColor(existingPVs),
    opacity: 1,
    lineWidth: 2,
    style: 'solid',
    showPoints: false,
    pointSize: 4,
    visible: true,
  });

  const handleAddPV = (pv: string, properties?: Partial<PenProperties>) => {
    try {
      if (selectedPVs().length >= maxPVs) {
        throw new Error(`Maximum number of PVs (${maxPVs}) reached`);
      }

      if (selectedPVs().some(existing => existing.name === pv)) {
        throw new Error(`PV "${pv}" already exists`);
      }

      const defaultPen = createDefaultPen(selectedPVs());
      const newPV: PVWithProperties = {
        name: pv,
        pen: {
          ...defaultPen,
          ...properties,
        },
      };

      setSelectedPVs(pvs => [...pvs, newPV]);
      
      if (newPV.pen.visible) {
        setVisiblePVs(prev => {
          const next = new Set(prev);
          next.add(pv);
          return next;
        });
      }
    } catch (error) {
      onError?.(error instanceof Error ? error : new Error(String(error)));
    }
  };

  const handleRemovePV = (pv: string) => {
    setSelectedPVs(pvs => pvs.filter(p => p.name !== pv));
    setVisiblePVs(prev => {
      const next = new Set(prev);
      next.delete(pv);
      return next;
    });
  };

  const handleUpdatePV = (pv: string, properties: Partial<PenProperties>) => {
    setSelectedPVs(pvs => 
      pvs.map(p => p.name === pv ? {
        ...p,
        pen: { ...p.pen, ...properties }
      } : p)
    );

    if (properties.visible !== undefined) {
      setVisiblePVs(prev => {
        const next = new Set(prev);
        if (properties.visible) {
          next.add(pv);
        } else {
          next.delete(pv);
        }
        return next;
      });
    }
  };

  const handleVisibilityChange = (pvName: string, isVisible: boolean) => {
    handleUpdatePV(pvName, { visible: isVisible });
  };

  // Effect to sync with initialPVs changes
  createEffect(() => {
    if (initialPVs.length > 0) {
      setSelectedPVs(initialPVs);
      setVisiblePVs(new Set(
        initialPVs.filter(pv => pv.pen?.visible !== false).map(pv => pv.name)
      ));
    }
  });

  return {
    selectedPVs,
    visiblePVs,
    handleAddPV,
    handleRemovePV,
    handleUpdatePV,
    handleVisibilityChange,
  };
}