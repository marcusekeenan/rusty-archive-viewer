import { invoke } from "@tauri-apps/api/tauri";
import type { Meta, Point, PVData, PVMetadata } from '../types';

interface FetchParams {
    [key: string]: string[] | number;
    pvs: string[];
    from: number;
    to: number;
}

export async function fetchData(
    pvs: string[],
    start: Date,
    end: Date
): Promise<PVData[]> {
    const params: FetchParams = {
        pvs,
        from: dateToUnixSeconds(start),
        to: dateToUnixSeconds(end),
    };
    
    try {
        return await invoke<PVData[]>("fetch_data", params);
    } catch (error) {
        console.error("Error fetching data:", error);
        throw error;
    }
}

export async function fetchLatest(pv: string): Promise<Point> {
    try {
        return await invoke<Point>("fetch_latest", { pv });
    } catch (error) {
        console.error("Error fetching latest data:", error);
        throw error;
    }
}

export async function getPVMetadata(pv: string): Promise<PVMetadata> {
    try {
        const meta = await invoke<Meta>("get_pv_metadata", { pv });
        return { name: pv, ...meta };
    } catch (error) {
        console.error("Error fetching PV metadata:", error);
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

// Time utility functions
const dateToUnixSeconds = (date: Date): number => Math.floor(date.getTime() / 1000);

export const formatTimestamp = (timestamp: number): string => 
    new Date(timestamp * 1000).toISOString();

export const getCurrentTimestamp = (): number => 
    dateToUnixSeconds(new Date());

export const pointToTimestamp = (point: Point): number => 
    point.secs * 1000 + point.nanos / 1_000_000;