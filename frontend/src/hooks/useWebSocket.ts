import { useEffect, useRef, useState } from 'react';
import { useAuth } from '../contexts/AuthContext';

export type WebSocketMessage = {
  type: 'auth' | 'subscribe' | 'usage_update' | 'system_stats' | 'error' | 'success';
  token?: string;
  topics?: string[];
  api_key_id?: string;
  requests_today?: number;
  requests_this_month?: number;
  rate_limit_status?: any;
  total_requests_today?: number;
  total_requests_this_month?: number;
  active_connections?: number;
  message?: string;
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
  const [subscriptions, setSubscriptions] = useState<string[]>([]);

  const connect = () => {
    if (!token) return;

    try {
      // Use the same host but WebSocket protocol
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsHost = window.location.hostname === 'localhost' ? 'localhost:8081' : window.location.host;
      const wsUrl = `${wsProtocol}//${wsHost}/ws`;
      
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

      ws.onclose = () => {
        console.log('WebSocket disconnected');
        setIsConnected(false);
        wsRef.current = null;
        
        // Reconnect after 5 seconds if we have a token
        if (token) {
          reconnectTimeoutRef.current = setTimeout(connect, 5000);
        }
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setIsConnected(false);
      };

    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
    }
  };

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

  const disconnect = () => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    
    setIsConnected(false);
  };

  // Connect when we have a token
  useEffect(() => {
    if (token) {
      connect();
    } else {
      disconnect();
    }

    return () => {
      disconnect();
    };
  }, [token]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnect();
    };
  }, []);

  return {
    isConnected,
    lastMessage,
    sendMessage,
    subscribe,
    disconnect
  };
}