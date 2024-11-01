import { invoke } from "@tauri-apps/api";

// Type definitions matching Rust types
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

export interface ExtendedFetchOptions {
    operator?: string;
    timezone?: string;
    chart_width?: number;
    batch_size?: number;
    fetch_latest_metadata?: boolean;
    retired_pv_template?: string;
    do_not_chunk?: boolean;
    ca_count?: number;
    ca_how?: number;
    use_raw_processing?: boolean;
    format?: DataFormat;
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

// API Functions

/**
 * Fetches binned data for multiple PVs
 */
export async function fetchBinnedData(
    pvs: string[],
    from: Date,
    to: Date,
    options?: ExtendedFetchOptions
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        options,
    };

    try {
        return await invoke<NormalizedPVData[]>("fetch_binned_data", params);
    } catch (error) {
        console.error("Error fetching binned data:", error);
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
 * Gets data at a specific point in time for multiple PVs
 */
export async function getDataAtTime(
    pvs: string[],
    timestamp: Date,
    options?: ExtendedFetchOptions
): Promise<Record<string, PointValue>> {
    const params = {
        pvs,
        timestamp: Math.floor(timestamp.getTime() / 1000),
        options,
    };

    try {
        return await invoke<Record<string, PointValue>>("get_data_at_time", params);
    } catch (error) {
        console.error("Error getting data at time:", error);
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
    format: DataFormat,
    options?: ExtendedFetchOptions
): Promise<string> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        format,
        options,
    };

    try {
        return await invoke<string>("export_data", params);
    } catch (error) {
        console.error("Error exporting data:", error);
        throw error;
    }
}

/**
 * Fetches data with a specific operator
 */
export async function fetchDataWithOperator(
    pvs: string[],
    from: Date,
    to: Date,
    operator: string,
    options?: ExtendedFetchOptions
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        operator,
        options,
    };

    try {
        return await invoke<NormalizedPVData[]>("fetch_data_with_operator", params);
    } catch (error) {
        console.error("Error fetching data with operator:", error);
        throw error;
    }
}

/**
 * Fetches raw data without any processing
 */
export async function fetchRawData(
    pvs: string[],
    from: Date,
    to: Date
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
    };

    try {
        return await invoke<NormalizedPVData[]>("fetch_raw_data", params);
    } catch (error) {
        console.error("Error fetching raw data:", error);
        throw error;
    }
}

/**
 * Fetches optimized data based on time range and display width
 */
export async function fetchOptimizedData(
    pvs: string[],
    from: Date,
    to: Date,
    chartWidth: number
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        chart_width: chartWidth,
    };

    try {
        return await invoke<NormalizedPVData[]>("fetch_optimized_data", params);
    } catch (error) {
        console.error("Error fetching optimized data:", error);
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