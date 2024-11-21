// archiverApi.ts

import { invoke } from "@tauri-apps/api/tauri";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
    Meta,
    NormalizedPVData,
    PointValue,
    ExtendedFetchOptions,
    DataFormat,
    DetailedPVStatus,
    LiveUpdateConfig
} from './types';

export class LiveUpdateManager {
    private unlistenFn?: UnlistenFn;
    private isActive = false;
    
    async start(config: LiveUpdateConfig): Promise<void> {
        if (this.isActive) {
            await this.stop();
        }
        
        this.isActive = true;
        
        try {
            console.log("Starting live updates for PVs:", config.pvs);
            
            await invoke("start_live_updates", {
                pvs: config.pvs,
                updateIntervalMs: config.updateIntervalMs,
                timezone: config.timezone
            });
            
            this.unlistenFn = await listen<Record<string, PointValue>>(
                "live-update",
                (event) => {
                    if (this.isActive) {
                        console.log("Received update:", event.payload);
                        config.onData(event.payload);
                    }
                }
            );
        } catch (error) {
            this.isActive = false;
            throw error;
        }
    }
    
    async stop(): Promise<void> {
        console.log("Stopping LiveUpdateManager");
        this.isActive = false;
        
        try {
            await invoke("stop_live_updates");
            
            if (this.unlistenFn) {
                await this.unlistenFn();
                this.unlistenFn = undefined;
            }

            console.log("LiveUpdateManager stopped successfully");
        } catch (error) {
            console.error("Error in LiveUpdateManager stop:", error);
            if (this.unlistenFn) {
                try {
                    await this.unlistenFn();
                } catch (e) {
                    console.error("Error cleaning up listener:", e);
                }
                this.unlistenFn = undefined;
            }
            throw error;
        }
    }

    isRunning(): boolean {
        return this.isActive;
    }
}

export async function fetchData(
    pvs: string[],
    start: Date,
    end: Date,
    options: ExtendedFetchOptions = {}
): Promise<NormalizedPVData[]> {
    const params = {
        pvs,
        from: Math.floor(start.getTime() / 1000),
        to: Math.floor(end.getTime() / 1000),
        timezone: options.timezone || 'UTC',
        mode: options.mode || 'fixed',
        optimization: options.optimization || 'optimized',
        target_points: options.targetPoints || 1000,
    };
    
    try {
        return await invoke<NormalizedPVData[]>("fetch_data", params);
    } catch (error) {
        console.error("Error fetching data:", error);
        throw error;
    }
}

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

export async function getPVMetadata(pv: string): Promise<Meta> {
    try {
        return await invoke<Meta>("get_pv_metadata", { pv });
    } catch (error) {
        console.error("Error fetching PV metadata:", error);
        throw error;
    }
}

export async function exportData(
    pvs: string[],
    from: Date,
    to: Date,
    format: DataFormat,
    options: ExtendedFetchOptions = {}
): Promise<string> {
    const params = {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        format,
        ...options
    };

    try {
        return await invoke<string>("export_data", params);
    } catch (error) {
        console.error("Error exporting data:", error);
        throw error;
    }
}

export async function validatePVs(pvs: string[]): Promise<boolean[]> {
    try {
        return await invoke<boolean[]>("validate_pvs", { pvs });
    } catch (error) {
        console.error("Error validating PVs:", error);
        throw error;
    }
}

export async function getPVStatus(pvs: string[]): Promise<DetailedPVStatus[]> {
    try {
        return await invoke<DetailedPVStatus[]>("get_pv_status", { pvs });
    } catch (error) {
        console.error("Error getting PV status:", error);
        throw error;
    }
}

export async function testConnection(): Promise<boolean> {
    try {
        return await invoke<boolean>("test_connection");
    } catch (error) {
        console.error("Error testing connection:", error);
        throw error;
    }
}

export function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toISOString();
}

export function getCurrentTimestamp(): number {
    return Math.floor(Date.now() / 1000);
}