import { useContext } from 'react';
import { WebSocketContext, type WebSocketContextType } from '../contexts/WebSocketContext';

export function useWebSocketContext(): WebSocketContextType {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocketContext must be used within a WebSocketProvider');
  }
  return context;
}