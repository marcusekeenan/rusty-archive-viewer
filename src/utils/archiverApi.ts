import { invoke } from "@tauri-apps/api/tauri";
import { 
  DataFormat,
  ProcessingMode,
  UPlotData,
  PVMetadata,
} from '../types';

interface FetchDataParams {
  pvs: string[];
  from: number;
  to: number;
  mode?: { [key: string]: null };  // For untagged enum, send variant as key
  format: DataFormat;
}

export async function fetchData(
  pvs: string[],
  from: Date,
  to: Date,
  mode: ProcessingMode = ProcessingMode.Raw,
  format: DataFormat = DataFormat.Raw
): Promise<UPlotData> {
  const params: FetchDataParams = {
    pvs,
    from: Math.floor(from.getTime() / 1000),
    to: Math.floor(to.getTime() / 1000),
    mode: mode ? { [mode]: null } : undefined,  // Convert enum to { "Raw": null } format
    format,
  };

  console.log('Mode being sent:', mode);
  console.log('Sending request with params:', JSON.stringify(params, null, 2));

  try {
    const response = await invoke<UPlotData>('fetch_data', { params });
    console.log('Received response:', JSON.stringify(response, null, 2));

    if (!response || !Array.isArray(response.timestamps) || !Array.isArray(response.series)) {
      throw new Error('Invalid or empty response from server');
    }

    return response;
  } catch (error) {
    console.error('Error fetching data:', error);
    throw error;
  }
}

// Rest of the file remains the same
export async function getPVMetadata(pvName: string): Promise<PVMetadata> {
  if (!pvName) {
    throw new Error('PV name is required');
  }

  try {
    const response = await invoke<PVMetadata>('get_pv_metadata', { pv: pvName });
    if (!response || typeof response !== 'object') {
      throw new Error('Invalid metadata response');
    }
    return response;
  } catch (error) {
    console.error(`Error fetching metadata for ${pvName}:`, error);
    throw error;
  }
}

export async function testConnection(format: DataFormat = DataFormat.Raw): Promise<boolean> {
  try {
    return await invoke<boolean>("test_connection", { format });
  } catch (error) {
    console.error('Error testing connection:', error);
    return false;
  }
}

export function getCurrentTimestamp(): number {
  return Math.floor(Date.now() / 1000);
}