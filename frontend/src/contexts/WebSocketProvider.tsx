import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { useAuth } from '../hooks/useAuth';
import { WebSocketContext, type WebSocketMessage } from './WebSocketContext';

interface WebSocketProviderProps {
  children: ReactNode;
}

export function WebSocketProvider({ children }: WebSocketProviderProps) {
  const { token } = useAuth();
  const [isConnected, setIsConnected] = useState(false);
  const [lastMessage, setLastMessage] = useState<WebSocketMessage | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const isConnectingRef = useRef(false);
  const [, setSubscriptions] = useState<string[]>([]);

  const disconnect = () => {
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    
    if (wsRef.current) {
      wsRef.current.close(1000, 'Provider disconnected');
      wsRef.current = null;
    }
    
    isConnectingRef.current = false;
    setIsConnected(false);
  };

  const connect = () => {
    if (!token) {
      return;
    }

    if (isConnectingRef.current) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.CONNECTING) {
      return;
    }

    try {
      isConnectingRef.current = true;
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsHost = window.location.hostname === 'localhost' ? 'localhost:8081' : window.location.host;
      const wsUrl = `${wsProtocol}//${wsHost}/ws`;
      
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        isConnectingRef.current = false;
        setIsConnected(true);
        
        // Authenticate immediately
        ws.send(JSON.stringify({
          type: 'auth',
          token: token
        }));
      };

      ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          setLastMessage(message);
          
          // Auto-subscribe after authentication
          if (message.type === 'success' && message.message === 'Authentication successful') {
            const defaultTopics = ['usage', 'system'];
            setSubscriptions(defaultTopics);
            ws.send(JSON.stringify({
              type: 'subscribe',
              topics: defaultTopics
            }));
          }
        } catch (error) {
          console.error('WebSocket Provider: Failed to parse message:', error);
        }
      };

      ws.onclose = (event) => {
        isConnectingRef.current = false;
        setIsConnected(false);
        
        if (wsRef.current === ws) {
          wsRef.current = null;
        }
        
        // Only reconnect for unexpected closures
        if (event.code !== 1000 && token) {
          reconnectTimeoutRef.current = setTimeout(() => {
            if (token && !wsRef.current) {
              connect();
            }
          }, 5000);
        }
      };

      ws.onerror = (error) => {
        console.error('WebSocket Provider: Error:', error);
        isConnectingRef.current = false;
        setIsConnected(false);
        
        if (wsRef.current === ws) {
          wsRef.current = null;
        }
      };

    } catch (error) {
      console.error('WebSocket Provider: Failed to create connection:', error);
      isConnectingRef.current = false;
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

  // Single effect to manage connection
  useEffect(() => {
    if (token) {
      connect();
    } else {
      disconnect();
    }

    // Cleanup on unmount or token change
    return () => {
      disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]); // Only depend on token, connect is stable

  return (
    <WebSocketContext.Provider value={{
      isConnected,
      lastMessage,
      sendMessage,
      subscribe
    }}>
      {children}
    </WebSocketContext.Provider>
  );
}