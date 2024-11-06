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
    chart_width?: number;
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

/**
 * Main data fetching function - backend handles optimization
 */
export async function fetchData(
    pvs: string[],
    start: Date,
    end: Date,
    chartWidth: number,
    timezone: string,
): Promise<NormalizedPVData[]> {
    // Convert parameters to snake_case for Tauri
    const params = {
        pvs,
        from: Math.floor(start.getTime() / 1000),
        to: Math.floor(end.getTime() / 1000),
        chartWidth: chartWidth,  // Use snake_case for Tauri command
        timezone
    };
    console.log("the timezone is from api", timezone);
    try {
        return await invoke<NormalizedPVData[]>("fetch_data", params);
    } catch (error) {
        console.error("Error fetching data:", error);
        throw error;
    }
}

export async function fetchLiveData(
    pvs: string[],
    timestamp?: Date,
    timezone?: string
): Promise<Record<string, PointValue>> {
    const params = {
        pvs,
        timestamp: timestamp ? Math.floor(timestamp.getTime() / 1000) : Math.floor(Date.now() / 1000),
        timezone
    };

    try {
        return await invoke<Record<string, PointValue>>("fetch_live_data", params);
    } catch (error) {
        console.error("Error getting live data:", error);
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

export interface LiveUpdateConfig {
    pvs: string[];
    bufferSize?: number;
    callback: (data: NormalizedPVData[]) => void;
}

/**
 * Starts live updates for the specified PVs
 */
export async function startLiveUpdates(config: LiveUpdateConfig): Promise<UnlistenFn> {
    try {
        // Start the live updates on the backend
        await invoke<void>("start_live_updates", {
            pvs: config.pvs,
            buffer_size: config.bufferSize
        });

        // Set up the event listener for updates
        return await listen<NormalizedPVData[]>("live-data-update", (event) => {
            config.callback(event.payload);
        });
    } catch (error) {
        console.error("Error starting live updates:", error);
        throw error;
    }
}

/**
 * Stops live updates
 */
export async function stopLiveUpdates(): Promise<void> {
    try {
        await invoke<void>("stop_live_updates");
    } catch (error) {
        console.error("Error stopping live updates:", error);
        throw error;
    }
}