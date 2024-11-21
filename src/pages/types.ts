// src/pages/types.ts

import {
  NormalizedPVData,
  PointValue,
  Meta,
  ProcessedPoint,
  Statistics,
  DetailedPVStatus,
  LiveUpdateConfig
} from '../types/api';

// Re-export types from api
export type {
  NormalizedPVData,
  PointValue,
  Meta,
  ProcessedPoint,
  Statistics,
  DetailedPVStatus,
  LiveUpdateConfig
};

// Time-related types
export interface TimeRange {
  start: Date;
  end: Date;
}

// ... (rest of the file remains the same)