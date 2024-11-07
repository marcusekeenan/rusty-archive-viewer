import { invoke } from "@tauri-apps/api/tauri";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

// Type definitions
export interface Meta {
    name: string;
    egu: string;
    description?: string;
    precision?: number;
    archive_parameters?: {
        sampling_period: number;
        sampling_method: string;
        last_modified: string;
    };
    display_limits?: {
        low: number;
        high: number;
    };
    alarm_limits?: {
        low: number;
        high: number;
        lolo: number;
        hihi: number;
    };
}

export interface ProcessedPoint {
    timestamp: number;
    severity: number;
    status: number;
    value: number;
    min: number;
    max: number;
    stddev: number;
    count: number;
}

export interface Statistics {
    mean: number;
    std_dev: number;
    min: number;
    max: number;
    count: number;
    first_timestamp: number;
    last_timestamp: number;
}

export interface NormalizedPVData {
    meta: Meta;
    data: ProcessedPoint[];
    statistics?: Statistics;
}

export interface FetchOptions {
    timezone?: string;
    chartWidth?: number;
}

export enum DataFormat {
    Json = "json",
    Csv = "csv",
    Raw = "raw",
    Matlab = "mat",
    Text = "txt",
    Svg = "svg"
}

export interface PointValue {
    secs: number;
    nanos?: number;
    val: number | number[] | string | Uint8Array;
    severity?: number;
    status?: number;
}

export interface PVStatus {
    name: string;
    connected: boolean;
    last_event_time?: number;
    last_status?: string;
    archived: boolean;
    error_count: number;
    last_error?: string;
}

export interface LiveUpdateConfig {
    pvs: string[];
    updateIntervalMs: number;
    timezone?: string;
    onData: (data: Record<string, PointValue>) => void;
    onError?: (error: string) => void;
}

/**
 * Manages real-time data updates for a window
 */
export class LiveUpdateManager {
    private unlistenFn?: UnlistenFn;
    private isActive = false;
    
    async start(config: LiveUpdateConfig): Promise<void> {
        if (this.isActive) {
            throw new Error("Live updates already running");
        }
        
        this.isActive = true;
        
        try {
            // Start the backend polling
            await invoke("start_live_updates", {
                pvs: config.pvs,
                updateIntervalMs: config.updateIntervalMs,
                timezone: config.timezone
            });
            
            // Listen for updates
            this.unlistenFn = await listen<Record<string, PointValue>>(
                "live-update",
                (event) => {
                    if (this.isActive) {
                        config.onData(event.payload);
                    }
                }
            );
            
            // Listen for errors if handler provided
            if (config.onError) {
                await listen("live-update-error", (event) => {
                    if (this.isActive && config.onError) {
                        config.onError(event.payload as string);
                    }
                });
            }
        } catch (error) {
            this.isActive = false;
            throw error;
        }
    }
    
    async stop(): Promise<void> {
        try {
            console.log("LiveUpdateManager: Stopping...");
            if (this.unlistenFn) {
                await this.unlistenFn();
                this.unlistenFn = undefined;
                console.log("LiveUpdateManager: Unlisten function cleared");
            }
            await invoke("stop_live_updates");
            console.log("LiveUpdateManager: Backend stopped");
        } catch (error) {
            console.error("LiveUpdateManager: Error during stop:", error);
            throw error;
        }
    }
}

/**
 * Fetches historical data with automatic optimization
 */
export async function fetchData(
    pvs: string[],
    start: Date,
    end: Date,
    chartWidth: number,
    timezone: string,
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(start.getTime() / 1000),
        to: Math.floor(end.getTime() / 1000),
        chartWidth, // Use camelCase for Tauri
        timezone
    };
    
    try {
        return await invoke<NormalizedPVData[]>("fetch_data", params);
    } catch (error) {
        console.error("Error fetching data:", error);
        throw error;
    }
}

/**
 * Fetches data at a specific timestamp
 */
export async function fetchDataAtTime(
    pvs: string[],
    timestamp?: Date,
    timezone?: string
): Promise<Record<string, PointValue>> {
    try {
        const params = {
            pvs,
            timestamp: timestamp ? Math.floor(timestamp.getTime() / 1000) : undefined,
            timezone
        };

        return await invoke<Record<string, PointValue>>("fetch_data_at_time", params);
    } catch (error) {
        console.error("Error fetching data at time:", error);
        throw error;
    }
}

/**
 * Fetches metadata for a PV
 */
export async function getPVMetadata(pv: string): Promise<Meta> {
    try {
        return await invoke<Meta>("get_pv_metadata", { pv });
    } catch (error) {
        console.error("Error fetching PV metadata:", error);
        throw error;
    }
}

/**
 * Exports data in various formats
 */
export async function exportData(
    pvs: string[],
    from: Date,
    to: Date,
    format: DataFormat
): Promise<string> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        format
    };

    try {
        return await invoke<string>("export_data", params);
    } catch (error) {
        console.error("Error exporting data:", error);
        throw error;
    }
}

/**
 * Validates PV names
 */
export async function validatePVs(pvs: string[]): Promise<boolean[]> {
    try {
        return await invoke<boolean[]>("validate_pvs", { pvs });
    } catch (error) {
        console.error("Error validating PVs:", error);
        throw error;
    }
}

/**
 * Gets status information for PVs
 */
export async function getPVStatus(pvs: string[]): Promise<PVStatus[]> {
    try {
        return await invoke<PVStatus[]>("get_pv_status", { pvs });
    } catch (error) {
        console.error("Error getting PV status:", error);
        throw error;
    }
}

/**
 * Tests connection to the archiver
 */
export async function testConnection(): Promise<boolean> {
    try {
        return await invoke<boolean>("test_connection");
    } catch (error) {
        console.error("Error testing connection:", error);
        throw error;
    }
}

// Helper functions
export function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toISOString();
}

export function getCurrentTimestamp(): number {
    return Math.floor(Date.now() / 1000);
}