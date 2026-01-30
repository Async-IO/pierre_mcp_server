// ABOUTME: WebSocket context for real-time chat streaming
// ABOUTME: Manages WebSocket connection lifecycle and message streaming for conversations

import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  useRef,
  type ReactNode,
} from 'react';
import { chatApi } from '../services/api';

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

interface StreamingMessage {
  conversationId: string;
  content: string;
  isComplete: boolean;
}

interface WebSocketContextType {
  status: ConnectionStatus;
  streamingMessage: StreamingMessage | null;
  connect: (conversationId: string) => void;
  disconnect: () => void;
  sendMessage: (content: string) => void;
}

const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined);

interface WebSocketProviderProps {
  children: ReactNode;
}

export function WebSocketProvider({ children }: WebSocketProviderProps) {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  const [streamingMessage, setStreamingMessage] = useState<StreamingMessage | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const conversationIdRef = useRef<string | null>(null);

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    conversationIdRef.current = null;
    setStatus('disconnected');
    setStreamingMessage(null);
  }, []);

  const connect = useCallback((conversationId: string) => {
    // Disconnect any existing connection
    if (wsRef.current) {
      disconnect();
    }

    conversationIdRef.current = conversationId;
    setStatus('connecting');

    const wsUrl = chatApi.getWebSocketUrl(conversationId);
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      setStatus('connected');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);

        if (data.type === 'stream') {
          // Handle streaming content
          setStreamingMessage((prev) => ({
            conversationId,
            content: (prev?.content || '') + (data.content || ''),
            isComplete: false,
          }));
        } else if (data.type === 'complete') {
          // Mark streaming as complete
          setStreamingMessage((prev) =>
            prev ? { ...prev, isComplete: true } : null
          );
        } else if (data.type === 'error') {
          console.error('WebSocket error:', data.message);
          setStatus('error');
        }
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      setStatus('error');
    };

    ws.onclose = () => {
      setStatus('disconnected');
    };

    wsRef.current = ws;
  }, [disconnect]);

  const sendMessage = useCallback((content: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      console.error('WebSocket is not connected');
      return;
    }

    // Clear previous streaming message
    setStreamingMessage(null);

    // Send the message
    wsRef.current.send(JSON.stringify({
      type: 'message',
      content,
    }));
  }, []);

  const value: WebSocketContextType = {
    status,
    streamingMessage,
    connect,
    disconnect,
    sendMessage,
  };

  return (
    <WebSocketContext.Provider value={value}>
      {children}
    </WebSocketContext.Provider>
  );
}

export function useWebSocket(): WebSocketContextType {
  const context = useContext(WebSocketContext);
  if (context === undefined) {
    throw new Error('useWebSocket must be used within a WebSocketProvider');
  }
  return context;
}
