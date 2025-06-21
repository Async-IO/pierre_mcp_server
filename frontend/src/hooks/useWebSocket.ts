import { useEffect, useRef, useState, useCallback } from 'react';
import { useAuth } from './useAuth';

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

export type UseWebSocketReturn = {
  isConnected: boolean;
  lastMessage: WebSocketMessage | null;
  sendMessage: (message: WebSocketMessage) => void;
  subscribe: (topics: string[]) => void;
  disconnect: () => void;
};

export function useWebSocket(): UseWebSocketReturn {
  const { token } = useAuth();
  const [isConnected, setIsConnected] = useState(false);
  const [lastMessage, setLastMessage] = useState<WebSocketMessage | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastConnectAttemptRef = useRef<number>(0);
  const [, setSubscriptions] = useState<string[]>([]);

  const connect = useCallback(() => {
    if (!token) {
      console.log('WebSocket: No token available, skipping connection');
      return;
    }

    // Debounce connection attempts (prevent rapid reconnects)
    const now = Date.now();
    if (now - lastConnectAttemptRef.current < 1000) {
      console.log('WebSocket: Debouncing connection attempt');
      return;
    }
    lastConnectAttemptRef.current = now;

    // Don't connect if already connected
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      console.log('WebSocket: Already connected');
      return;
    }

    // Don't connect if already connecting
    if (wsRef.current?.readyState === WebSocket.CONNECTING) {
      console.log('WebSocket: Already connecting');
      return;
    }

    try {
      // Use the same host but WebSocket protocol
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      // Get WebSocket URL from environment or use default based on current location
      const wsBaseUrl = import.meta.env.VITE_WS_BASE_URL;
      const wsUrl = wsBaseUrl || `${wsProtocol}//${window.location.hostname === 'localhost' ? 'localhost:8081' : window.location.host}/ws`;
      
      console.log('WebSocket: Connecting to', wsUrl);
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        
        // Authenticate immediately after connection
        ws.send(JSON.stringify({
          type: 'auth',
          token: token
        }));
      };

      ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          setLastMessage(message);
          
          // Auto-subscribe to default topics after successful authentication
          if (message.type === 'success' && message.message === 'Authentication successful') {
            const defaultTopics = ['usage', 'system'];
            setSubscriptions(defaultTopics);
            ws.send(JSON.stringify({
              type: 'subscribe',
              topics: defaultTopics
            }));
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      ws.onclose = (event) => {
        console.log('WebSocket disconnected, code:', event.code, 'reason:', event.reason);
        setIsConnected(false);
        
        // Only clear ref if this is the current connection
        if (wsRef.current === ws) {
          wsRef.current = null;
        }
        
        // Only reconnect for unexpected closures (not normal close)
        if (event.code !== 1000 && token && wsRef.current === null) {
          console.log('WebSocket: Scheduling reconnect in 5 seconds');
          reconnectTimeoutRef.current = setTimeout(connect, 5000);
        }
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setIsConnected(false);
        
        // Only set to null if this is the current connection
        if (wsRef.current === ws) {
          wsRef.current = null;
        }
      };

    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
    }
  }, [token]);

  const sendMessage = (message: WebSocketMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    }
  };

  const subscribe = (topics: string[]) => {
    setSubscriptions(topics);
    sendMessage({
      type: 'subscribe',
      topics
    });
  };

  const disconnect = useCallback(() => {
    console.log('WebSocket: Disconnecting');
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    
    if (wsRef.current) {
      wsRef.current.close(1000, 'User disconnected'); // Normal closure
      wsRef.current = null;
    }
    
    // Reset connection attempt tracking
    lastConnectAttemptRef.current = 0;
    setIsConnected(false);
  }, []);

  // Connect when we have a token
  useEffect(() => {
    let mounted = true;
    
    if (token && mounted) {
      connect();
    } else if (!token) {
      disconnect();
    }

    return () => {
      mounted = false;
      disconnect();
    };
  }, [token, connect, disconnect]);

  return {
    isConnected,
    lastMessage,
    sendMessage,
    subscribe,
    disconnect
  };
}