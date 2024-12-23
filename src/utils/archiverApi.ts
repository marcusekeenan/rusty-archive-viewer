import { invoke } from "@tauri-apps/api/tauri";
import { 
  DataFormat,
  ProcessingMode,
  UPlotData,
  PVMetadata,
  Meta,
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
export const getPVMetadata = async (pv: string): Promise<PVMetadata> => {
  try {
      // Get the raw response as a string
      const rawResponse = await invoke<string>('get_pv_metadata', { pv });
      
      // Parse the JSON ourselves
      const responseData = JSON.parse(rawResponse);

      // Return metadata in the expected format
      return {
          name: pv,
          EGU: responseData.EGU || responseData.units || "Value",
          PREC: Number(responseData.PREC || responseData.precision || "2"),
          DESC: responseData.DESC || pv,
          LOPR: Number(responseData.LOPR || responseData.lowerDisplayLimit || "-100"),
          HOPR: Number(responseData.HOPR || responseData.upperDisplayLimit || "100"),
          HIGH: Number(responseData.HIGH || responseData.upperWarningLimit || "100"),
          LOW: Number(responseData.LOW || responseData.lowerWarningLimit || "-100"),
          HIHI: Number(responseData.HIHI || responseData.upperAlarmLimit || "100"),
          LOLO: Number(responseData.LOLO || responseData.lowerAlarmLimit || "-100"),
          DRVH: Number(responseData.DRVH || responseData.upperCtrlLimit || "100"),
          DRVL: Number(responseData.DRVL || responseData.lowerCtrlLimit || "-100")
      };
  } catch (error) {
      console.error(`Error fetching metadata for ${pv}:`, error);
      return {
          name: pv,
          EGU: "Value",
          PREC: 2,
          DESC: pv,
          LOPR: -100,
          HOPR: 100,
          HIGH: 100,
          LOW: -100,
          HIHI: 100,
          LOLO: -100,
          DRVH: 100,
          DRVL: -100
      };
  }
};

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