export interface ChartDataset {
  label: string;
  data: number[];
  backgroundColor?: string | string[];
  borderColor?: string | string[];
  borderWidth?: number;
  tension?: number;
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
    };
    y?: {
      display?: boolean;
      beginAtZero?: boolean;
      title?: {
        display?: boolean;
        text?: string;
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