import { vi, describe, it, expect, beforeEach } from 'vitest';
import { invoke } from "@tauri-apps/api/tauri";
import { fetchData, fetchLatest, getPVMetadata, testConnection, pointToTimestamp } from './archiverApi';
import type { Point, PVData, Meta } from '../types';

vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn()
}));

describe('archiverApi', () => {
  const mockMeta: Meta = {
    name: "ROOM:LI30:1:OUTSIDE_TEMP",
    DRVH: "230.0",
    EGU: "DegF",
    HIGH: "203.0",
    HIHI: "212.0",
    DRVL: "14.0",
    PREC: "1.0",
    LOW: "41.0",
    LOLO: "32.0",
    LOPR: "14.0",
    HOPR: "230.0",
    NELM: "1",
    DESC: "OUTSIDET"
  };

  const mockPVData: PVData[] = [{
    meta: mockMeta,
    data: [
      {
        secs: 1732684734,
        val: 41.97542953491211,
        nanos: 701716897,
        severity: 0,
        status: 0
      },
      {
        secs: 1732684745,
        val: 41.967159271240234,
        nanos: 701670512,
        severity: 0,
        status: 0
      }
    ]
  }];

  const mockPoint: Point = {
    secs: 1732684734,
    val: 41.97542953491211,
    nanos: 701716897,
    severity: 0,
    status: 0
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('fetchData', () => {
    it('should fetch temperature data correctly', async () => {
      vi.mocked(invoke).mockResolvedValue(mockPVData);

      const result = await fetchData(
        ['ROOM:LI30:1:OUTSIDE_TEMP'],
        new Date('2024-01-01T00:00:00Z'),
        new Date('2024-01-01T01:00:00Z')
      );

      expect(invoke).toHaveBeenCalledWith('fetch_data', {
        pvs: ['ROOM:LI30:1:OUTSIDE_TEMP'],
        from: 1704067200,
        to: 1704070800
      });

      expect(result[0].meta).toEqual(mockMeta);
      expect(result[0].data[0]).toMatchObject({
        secs: expect.any(Number),
        nanos: expect.any(Number),
        val: expect.any(Number),
        severity: expect.any(Number),
        status: expect.any(Number)
      });
    });

    it('should handle multiple PVs', async () => {
      const multiPVData = [
        mockPVData[0],
        {
          meta: { ...mockMeta, name: 'CPT:PSI5:5205:PRESS', EGU: 'PSI' },
          data: mockPVData[0].data
        }
      ];

      vi.mocked(invoke).mockResolvedValue(multiPVData);

      const result = await fetchData(
        ['ROOM:LI30:1:OUTSIDE_TEMP', 'CPT:PSI5:5205:PRESS'],
        new Date('2024-01-01T00:00:00Z'),
        new Date('2024-01-01T01:00:00Z')
      );

      expect(result).toHaveLength(2);
      expect(result[1].meta.EGU).toBe('PSI');
    });

    it('should handle errors', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Network error'));

      await expect(fetchData(
        ['ROOM:LI30:1:OUTSIDE_TEMP'],
        new Date(),
        new Date()
      )).rejects.toThrow('Network error');
    });
  });

  describe('fetchLatest', () => {
    it('should return latest point data', async () => {
      vi.mocked(invoke).mockResolvedValue(mockPoint);

      const result = await fetchLatest('ROOM:LI30:1:OUTSIDE_TEMP');

      expect(invoke).toHaveBeenCalledWith('fetch_latest', { 
        pv: 'ROOM:LI30:1:OUTSIDE_TEMP' 
      });
      expect(result).toEqual(mockPoint);
      expect(result.val).toBeCloseTo(41.975, 3);
    });

    it('should handle invalid PV', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('PV not found'));
      await expect(fetchLatest('INVALID:PV')).rejects.toThrow('PV not found');
    });
  });

  describe('getPVMetadata', () => {
    it('should return complete metadata', async () => {
      vi.mocked(invoke).mockResolvedValue(mockMeta);

      const result = await getPVMetadata('ROOM:LI30:1:OUTSIDE_TEMP');

      expect(invoke).toHaveBeenCalledWith('get_pv_metadata', { 
        pv: 'ROOM:LI30:1:OUTSIDE_TEMP' 
      });
      
      expect(result).toMatchObject({
        name: 'ROOM:LI30:1:OUTSIDE_TEMP',
        EGU: 'DegF',
        PREC: '1.0',
        DESC: 'OUTSIDET',
        HOPR: '230.0',
        LOPR: '14.0',
        HIGH: '203.0',
        LOW: '41.0',
        HIHI: '212.0',
        LOLO: '32.0'
      });
    });

    it('should handle missing metadata fields', async () => {
      const partialMeta = {
        name: 'TEST:PV',
        EGU: 'DegF'
      };
      
      vi.mocked(invoke).mockResolvedValue(partialMeta);
      const result = await getPVMetadata('TEST:PV');
      expect(result.EGU).toBe('DegF');
      expect(result.PREC).toBeUndefined();
    });
  });

  describe('testConnection', () => {
    it('should verify connection', async () => {
      vi.mocked(invoke).mockResolvedValue(true);
      const result = await testConnection();
      expect(invoke).toHaveBeenCalledWith('test_connection');
      expect(result).toBe(true);
    });

    it('should handle connection failure', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Connection failed'));
      await expect(testConnection()).rejects.toThrow('Connection failed');
    });
  });

  describe('pointToTimestamp', () => {
    it('should convert point to millisecond timestamp', () => {
      const timestamp = pointToTimestamp(mockPoint);
      expect(timestamp).toBe(1732684734701.7169); 
    });

    it('should handle zero nanoseconds', () => {
      const point = { ...mockPoint, nanos: 0 };
      const timestamp = pointToTimestamp(point);
      expect(timestamp).toBe(1732684734000);
    });
  });
});