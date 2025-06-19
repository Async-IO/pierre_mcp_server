import { createContext } from 'react';

export type WebSocketMessage = {
  type: 'auth' | 'subscribe' | 'usage_update' | 'system_stats' | 'error' | 'success' | 'request_update';
  token?: string;
  topics?: string[];
  api_key_id?: string;
  requests_today?: number;
  requests_this_month?: number;
  rate_limit_status?: Record<string, unknown>;
  total_requests_today?: number;
  total_requests_this_month?: number;
  active_connections?: number;
  message?: string;
  data?: unknown;
};

export interface WebSocketContextType {
  isConnected: boolean;
  lastMessage: WebSocketMessage | null;
  sendMessage: (message: WebSocketMessage) => void;
  subscribe: (topics: string[]) => void;
}

export const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined);