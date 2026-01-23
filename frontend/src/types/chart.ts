// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

export interface ChartDataset {
  label: string;
  data: number[];
  backgroundColor?: string | string[];
  borderColor?: string | string[];
  borderWidth?: number;
  tension?: number;
  fill?: boolean | string;
  pointRadius?: number;
}

export interface ChartData {
  labels: string[];
  datasets: ChartDataset[];
}

export interface ChartOptions {
  responsive?: boolean;
  maintainAspectRatio?: boolean;
  plugins?: {
    legend?: {
      display?: boolean;
      position?: 'top' | 'bottom' | 'left' | 'right';
      labels?: {
        color?: string;
      };
    };
    title?: {
      display?: boolean;
      text?: string;
    };
  };
  scales?: {
    x?: {
      display?: boolean;
      title?: {
        display?: boolean;
        text?: string;
      };
      ticks?: {
        color?: string;
      };
      grid?: {
        color?: string;
      };
    };
    y?: {
      display?: boolean;
      beginAtZero?: boolean;
      title?: {
        display?: boolean;
        text?: string;
      };
      ticks?: {
        color?: string;
      };
      grid?: {
        color?: string;
      };
    };
  };
}

export interface ApiUsageData {
  dates: string[];
  usage: number[];
}

export interface RateLimitData {
  tier: string;
  current: number;
  limit: number;
  percentage: number;
}

export interface TimeSeriesPoint {
  date: string;
  request_count: number;
  error_count: number;
}

export interface TopTool {
  tool_name: string;
  request_count: number;
  average_response_time?: number;
  success_rate?: number;
}

export interface AnalyticsData {
  time_series: TimeSeriesPoint[];
  top_tools: TopTool[];
  total_requests: number;
  total_errors: number;
  avg_requests_per_day: number;
  unique_days_active: number;
  error_rate?: number;
  average_response_time?: number;
}