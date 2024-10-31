// src/utils/archiverApi.ts

import { invoke } from "@tauri-apps/api";

// Define the types for FetchOptions and NormalizedPVData
export type FetchOptions = {
  operator?: string | null;
  timezone?: string;
  chartWidth?: number;
};

export type NormalizedPVData = {
  name: string;
  data: number[]; // Adjust based on actual data type
  meta: any;      // Adjust based on actual metadata structure
};

// Function to fetch binned data
export async function fetchBinnedData(
  pvs: string[],
  from: Date,
  to: Date,
  options?: FetchOptions
): Promise<NormalizedPVData[]> {
  const params = {
    pvs,
    from: Math.floor(from.getTime() / 1000), // Convert to Unix timestamp
    to: Math.floor(to.getTime() / 1000),
    options,
  };

  console.log("Invoking fetch_binned_data with params:", params);

  try {
    const result = await invoke<NormalizedPVData[]>("fetch_binned_data", params);
    console.log("Received data:", result);
    return result;
  } catch (error) {
    console.error("Error fetching binned data:", error);
    throw error;
  }
}

// Function to fetch archiver data
export async function fetchArchiverData(
  pv: string,
  from: Date,
  to: Date,
  options?: FetchOptions
): Promise<NormalizedPVData> {
  const params = {
    pv,
    from: Math.floor(from.getTime() / 1000),
    to: Math.floor(to.getTime() / 1000),
    options,
  };

  console.log("Invoking fetch_archiver_data with params:", params);

  try {
    const result = await invoke<NormalizedPVData>("fetch_archiver_data", params);
    console.log("Received archiver data:", result);
    return result;
  } catch (error) {
    console.error("Error fetching archiver data:", error);
    throw error;
  }
}

// Function to set archiver URL
export async function setArchiverURL(url: string): Promise<void> {
  console.log("Invoking set_archiver_url_command with url:", url);

  try {
    await invoke("set_archiver_url_command", { url });
    console.log("Archiver URL set successfully.");
  } catch (error) {
    console.error("Error setting archiver URL:", error);
    throw error;
  }
}
