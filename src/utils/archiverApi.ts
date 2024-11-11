import { createStore } from "solid-js/store";
import { invoke } from "@tauri-apps/api/tauri";

// Type Definitions
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

export interface PenProperties {
    color: string;
    visible: boolean;
    displayName?: string;
    yAxis?: number;
    opacity: number;
    lineWidth: number;
    style: 'solid' | 'dashed' | 'dotted';
    showPoints: boolean;
    pointSize: number;
}

export interface PVWithProperties {
    name: string;
    pen: PenProperties;
}

export interface YAxisConfig {
    id: number;
    unit: string;
    position: 'left' | 'right';
    pvs: string[];
    range?: {
        min: number;
        max: number;
        autoScale: boolean;
    };
}

export interface TimeWindow {
    start: Date;
    end: Date;
}

export interface ChartConfig {
    width: number;
    height: number;
    timeZone: string;
    mode: 'historical' | 'live';
    updateRate: number;
}

export enum DataFormat {
    Json = "json",
    Csv = "csv",
    Raw = "raw",
    Matlab = "mat",
    Text = "txt",
    Svg = "svg"
}

// Store Types
interface AppState {
    pvConfigs: PVWithProperties[];
    timeWindow: TimeWindow;
    chartConfig: ChartConfig;
    isLive: boolean;
    data: NormalizedPVData[];
    yAxes: YAxisConfig[];
    error: string | null;
    loading: boolean;
    lastRefresh: Date | null;
}

// Initial state
const defaultState: AppState = {
    pvConfigs: [],
    timeWindow: {
        start: new Date(Date.now() - 3600000), // 1 hour ago
        end: new Date(),
    },
    chartConfig: {
        width: 1000,
        height: 600,
        timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        mode: 'historical',
        updateRate: 1000,
    },
    isLive: false,
    data: [],
    yAxes: [{ id: 0, unit: '', position: 'left', pvs: [] }],
    error: null,
    loading: false,
    lastRefresh: null,
};

function createArchiveStore() {
    const [state, setState] = createStore<AppState>(defaultState);
    
    // Local variables
    let liveUpdateInterval: number | null = null;

    // Load saved state
    try {
        const saved = localStorage.getItem('archiveViewerState');
        if (saved) {
            const savedState = JSON.parse(saved);
            setState(state => ({ ...state, ...savedState }));
        }
    } catch (error) {
        console.error('Error loading saved state:', error);
    }

    const saveState = () => {
        try {
            const saveData = {
                pvConfigs: state.pvConfigs,
                chartConfig: state.chartConfig,
                yAxes: state.yAxes,
            };
            localStorage.setItem('archiveViewerState', JSON.stringify(saveData));
        } catch (error) {
            console.error('Error saving state:', error);
        }
    };

    // Data fetching functions
    const fetchData = async (
        pvs: string[],
        start: Date,
        end: Date,
        chartWidth: number,
        timezone: string
    ): Promise<NormalizedPVData[]> => {
        setState('loading', true);
        setState('error', null);

        try {
            const data = await invoke<NormalizedPVData[]>('fetch_data', {
                pvs,
                from: Math.floor(start.getTime() / 1000),
                to: Math.floor(end.getTime() / 1000),
                chartWidth,
                timezone,
            });
            setState('data', data);
            setState('lastRefresh', new Date());
            return data;
        } catch (error) {
            setState('error', String(error));
            throw error;
        } finally {
            setState('loading', false);
        }
    };

    const fetchDataAtTime = async (
        pvs: string[],
        timestamp?: Date,
        timezone?: string
    ): Promise<Record<string, number>> => {
        try {
            const result = await invoke<Record<string, { val: number | number[] }>>('fetch_data_at_time', {
                pvs,
                timestamp: timestamp ? Math.floor(timestamp.getTime() / 1000) : undefined,
                timezone,
            });

            const simplifiedResult: Record<string, number> = {};
            for (const [pv, data] of Object.entries(result)) {
                if (Array.isArray(data.val)) {
                    simplifiedResult[pv] = data.val[0];
                } else {
                    simplifiedResult[pv] = data.val as number;
                }
            }

            return simplifiedResult;
        } catch (error) {
            console.error('Error fetching current values:', error);
            throw error;
        }
    };

    const startLiveUpdates = async () => {
        if (state.isLive || liveUpdateInterval) return;

        setState('isLive', true);
        setState('chartConfig', 'mode', 'live');

        const updateData = async () => {
            const pvs = state.pvConfigs.filter(pv => pv.pen.visible).map(pv => pv.name);
            if (pvs.length === 0) return;

            try {
                const currentValues = await fetchDataAtTime(
                    pvs,
                    new Date(),
                    state.chartConfig.timeZone
                );

                setState('data', prevData => {
                    return prevData.map(pvData => {
                        const value = currentValues[pvData.meta.name];
                        if (value === undefined) return pvData;

                        const newPoint: ProcessedPoint = {
                            timestamp: Date.now(),
                            severity: 0,
                            status: 0,
                            value,
                            min: value,
                            max: value,
                            stddev: 0,
                            count: 1,
                        };

                        const windowStart = state.timeWindow.start.getTime();
                        const filteredData = pvData.data
                            .filter(p => p.timestamp >= windowStart)
                            .slice(-state.chartConfig.width);

                        return {
                            ...pvData,
                            data: [...filteredData, newPoint],
                        };
                    });
                });
            } catch (error) {
                console.error('Live update error:', error);
            }
        };

        liveUpdateInterval = window.setInterval(
            updateData,
            state.chartConfig.updateRate
        );
    };

    const stopLiveUpdates = async () => {
        if (liveUpdateInterval) {
            clearInterval(liveUpdateInterval);
            liveUpdateInterval = null;
        }
        
        setState('isLive', false);
        setState('chartConfig', 'mode', 'historical');
    };

    const manageYAxis = (pvName: string, unit: string): number => {
        const existingAxis = state.yAxes.find(axis => axis.unit === unit);
        
        if (existingAxis) {
            if (!existingAxis.pvs.includes(pvName)) {
                setState('yAxes', axes => axes.map(axis => 
                    axis.id === existingAxis.id 
                        ? { ...axis, pvs: [...axis.pvs, pvName] }
                        : axis
                ));
            }
            return existingAxis.id;
        } else {
            const newAxis: YAxisConfig = {
                id: state.yAxes.length,
                unit,
                position: state.yAxes.length % 2 === 0 ? 'left' : 'right',
                pvs: [pvName],
                range: {
                    min: 0,
                    max: 100,
                    autoScale: true,
                },
            };
            setState('yAxes', yAxes => [...yAxes, newAxis]);
            return newAxis.id;
        }
    };

    // Create an object with all the functions and state
    return {
        state,
        setState,
        saveState,
        fetchData,
        fetchDataAtTime,
        startLiveUpdates,
        stopLiveUpdates,
        manageYAxis,
    };
}

// Export a singleton instance of the store
export const archiveStore = createArchiveStore();

// Export other utility functions
export const getPVMetadata = (pv: string): Promise<Meta> => {
    return invoke<Meta>('get_pv_metadata', { pv });
};

export const validatePVs = (pvs: string[]): Promise<boolean[]> => {
    return invoke<boolean[]>('validate_pvs', { pvs });
};

export const exportData = (
    pvs: string[],
    from: Date,
    to: Date,
    format: DataFormat
): Promise<string> => {
    return invoke<string>('export_data', {
        pvs,
        from: Math.floor(from.getTime() / 1000),
        to: Math.floor(to.getTime() / 1000),
        format,
    });
};

// Export utility functions
export const formatTimestamp = (timestamp: number): string => {
    return new Date(timestamp).toISOString();
};

export const getCurrentTimestamp = (): number => {
    return Math.floor(Date.now() / 1000);
};