// src/utils/colorUtils.ts

import type { PVWithProperties } from '../types';
import { PV_COLORS } from '../types';

/**
 * Get the next available color from the PV_COLORS array
 * @param existingPVs Current list of PVs with their colors
 * @returns The next available color
 */
export const getNextColor = (existingPVs: PVWithProperties[]): string => {
  const usedColors = new Set(existingPVs.map(pv => pv.pen.color));
  
  // First, try to find an unused color
  const availableColor = PV_COLORS.find(color => !usedColors.has(color));
  if (availableColor) {
    return availableColor;
  }
  
  // If all colors are used, cycle through them based on the number of existing PVs
  return PV_COLORS[existingPVs.length % PV_COLORS.length];
};

/**
 * Convert color to RGBA format
 * @param color Hex color string
 * @param opacity Opacity value (0-1)
 * @returns RGBA color string
 */
export const toRGBA = (color: string, opacity: number = 1): string => {
  // Handle shorthand hex colors (#RGB)
  if (color.length === 4) {
    color = `#${color[1]}${color[1]}${color[2]}${color[2]}${color[3]}${color[3]}`;
  }

  // Convert hex to RGB
  const r = parseInt(color.slice(1, 3), 16);
  const g = parseInt(color.slice(3, 5), 16);
  const b = parseInt(color.slice(5, 7), 16);

  return `rgba(${r}, ${g}, ${b}, ${opacity})`;
};

/**
 * Get contrasting text color (black or white) for a background color
 * @param backgroundColor Hex color string
 * @returns '#000000' for light backgrounds, '#ffffff' for dark backgrounds
 */
export const getContrastColor = (backgroundColor: string): string => {
  // Convert hex to RGB
  const r = parseInt(backgroundColor.slice(1, 3), 16);
  const g = parseInt(backgroundColor.slice(3, 5), 16);
  const b = parseInt(backgroundColor.slice(5, 7), 16);
  
  // Calculate relative luminance
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  
  return luminance > 0.5 ? '#000000' : '#ffffff';
};

/**
 * Get a color for a specific PV, creating one if it doesn't exist
 * @param pvName PV name
 * @param existingPVs Current list of PVs
 * @returns Color for the PV
 */
export const getColorForPV = (pvName: string, existingPVs: PVWithProperties[]): string => {
  const existingPV = existingPVs.find(pv => pv.name === pvName);
  if (existingPV) {
    return existingPV.pen.color;
  }
  return getNextColor(existingPVs);
};

/**
 * Generate a gradient scale of colors
 * @param startColor Starting hex color
 * @param endColor Ending hex color
 * @param steps Number of steps in the gradient
 * @returns Array of hex color strings
 */
export const generateColorScale = (startColor: string, endColor: string, steps: number): string[] => {
  const scale: string[] = [];
  
  const startRGB = {
    r: parseInt(startColor.slice(1, 3), 16),
    g: parseInt(startColor.slice(3, 5), 16),
    b: parseInt(startColor.slice(5, 7), 16)
  };
  
  const endRGB = {
    r: parseInt(endColor.slice(1, 3), 16),
    g: parseInt(endColor.slice(3, 5), 16),
    b: parseInt(endColor.slice(5, 7), 16)
  };

  for (let i = 0; i < steps; i++) {
    const factor = i / (steps - 1);
    const r = Math.round(startRGB.r + (endRGB.r - startRGB.r) * factor);
    const g = Math.round(startRGB.g + (endRGB.g - startRGB.g) * factor);
    const b = Math.round(startRGB.b + (endRGB.b - startRGB.b) * factor);
    
    scale.push(`#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`);
  }

  return scale;
};

/**
 * Validate a color string
 * @param color Color string to validate
 * @returns boolean indicating if color is valid
 */
export const isValidColor = (color: string): boolean => {
  if (!color) return false;
  
  // Check if it's a hex color
  if (color.startsWith('#')) {
    return /^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$/.test(color);
  }
  
  // Check if it's an RGB/RGBA color
  if (color.startsWith('rgb')) {
    return /^rgb(a)?\((\d{1,3},\s*){2}\d{1,3}(,\s*\d*\.?\d+)?\)$/.test(color);
  }
  
  return false;
};

/**
 * Color management class for maintaining color assignments
 */
export class ColorManager {
  private usedColors: Map<string, string> = new Map();
  private static instance: ColorManager;

  private constructor() {}

  static getInstance(): ColorManager {
    if (!ColorManager.instance) {
      ColorManager.instance = new ColorManager();
    }
    return ColorManager.instance;
  }

  assignColor(pvName: string, existingPVs: PVWithProperties[]): string {
    if (this.usedColors.has(pvName)) {
      return this.usedColors.get(pvName)!;
    }

    const newColor = getNextColor(existingPVs);
    this.usedColors.set(pvName, newColor);
    return newColor;
  }

  releaseColor(pvName: string): void {
    this.usedColors.delete(pvName);
  }

  getAssignedColors(): Map<string, string> {
    return new Map(this.usedColors);
  }

  reset(): void {
    this.usedColors.clear();
  }
}

export const colorManager = ColorManager.getInstance();