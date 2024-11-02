// colors.ts
// Color palette inspired by tailwind and other common charting libraries
export const PV_COLORS = [
  "#2563eb", // Blue
  "#dc2626", // Red
  "#16a34a", // Green
  "#9333ea", // Purple
  "#ea580c", // Orange
  "#0891b2", // Cyan
  "#db2777", // Pink
  "#854d0e", // Amber
  "#2E4053", // Dark Blue Gray
  "#7c3aed", // Violet
  "#059669", // Emerald
  "#d97706", // Yellow
  "#be123c", // Rose
  "#475569", // Slate
  "#6366f1", // Indigo
  "#b45309"  // Bronze
];

export function getNextColor(existingPVs: { pen: { color: string } }[]): string {
  // Get currently used colors
  const usedColors = new Set(existingPVs.map(pv => pv.pen.color));
  
  // Find first unused color
  const availableColor = PV_COLORS.find(color => !usedColors.has(color));
  
  if (availableColor) {
    return availableColor;
  }
  
  // If all colors are used, cycle through the palette
  return PV_COLORS[existingPVs.length % PV_COLORS.length];
}