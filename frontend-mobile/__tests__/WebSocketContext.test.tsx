// ABOUTME: Unit tests for WebSocketContext
// ABOUTME: Tests WebSocket connection lifecycle and message streaming

import React from 'react';
import { render, act, waitFor, fireEvent } from '@testing-library/react-native';
import { Text, TouchableOpacity } from 'react-native';
import { WebSocketProvider, useWebSocket } from '../src/contexts/WebSocketContext';
import { chatApi } from '../src/services/api';

// Mock the api service
jest.mock('../src/services/api', () => ({
  chatApi: {
    getWebSocketUrl: jest.fn(() => 'ws://localhost:8081/api/chat/ws/conv-123?token=jwt'),
  },
}));

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  static OPEN = 1;

  url: string;
  readyState: number = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: ((error: Event) => void) | null = null;
  onclose: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send = jest.fn();
  close = jest.fn(() => {
    this.readyState = 3; // CLOSED
    if (this.onclose) this.onclose();
  });

  // Helper to simulate connection
  simulateOpen() {
    this.readyState = 1; // OPEN
    if (this.onopen) this.onopen();
  }

  // Helper to simulate message
  simulateMessage(data: object) {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(data) });
    }
  }

  // Helper to simulate error
  simulateError(error: Event) {
    if (this.onerror) this.onerror(error);
  }

  static reset() {
    MockWebSocket.instances = [];
  }
}

// @ts-expect-error - Replacing global WebSocket with mock
global.WebSocket = MockWebSocket;

// Test component that uses the WebSocket context
function TestWebSocketConsumer() {
  const { status, streamingMessage, connect, disconnect, sendMessage } = useWebSocket();

  return (
    <>
      <Text testID="status">{status}</Text>
      <Text testID="streaming-content">
        {streamingMessage?.content || 'no-content'}
      </Text>
      <Text testID="streaming-complete">
        {streamingMessage?.isComplete ? 'complete' : 'not-complete'}
      </Text>
      <TouchableOpacity
        testID="connect-btn"
        onPress={() => connect('conv-123')}
      >
        <Text>Connect</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="disconnect-btn"
        onPress={disconnect}
      >
        <Text>Disconnect</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="send-btn"
        onPress={() => sendMessage('Hello')}
      >
        <Text>Send</Text>
      </TouchableOpacity>
    </>
  );
}

describe('WebSocketContext', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    MockWebSocket.reset();
  });

  describe('useWebSocket hook', () => {
    it('should throw error when used outside WebSocketProvider', () => {
      const consoleError = jest.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        render(<TestWebSocketConsumer />);
      }).toThrow('useWebSocket must be used within a WebSocketProvider');

      consoleError.mockRestore();
    });
  });

  describe('WebSocketProvider initialization', () => {
    it('should start in disconnected state', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      expect(getByTestId('status').children[0]).toBe('disconnected');
      expect(getByTestId('streaming-content').children[0]).toBe('no-content');
    });
  });

  describe('connect', () => {
    it('should update status to connecting when connect is called', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      expect(getByTestId('status').children[0]).toBe('connecting');
      expect(chatApi.getWebSocketUrl).toHaveBeenCalledWith('conv-123');
    });

    it('should update status to connected on open', async () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      // Simulate WebSocket open
      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      expect(getByTestId('status').children[0]).toBe('connected');
    });

    it('should disconnect existing connection before creating new one', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      // Connect first time
      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      const firstWs = MockWebSocket.instances[0];

      // Connect again
      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      expect(firstWs.close).toHaveBeenCalled();
      expect(MockWebSocket.instances.length).toBe(2);
    });
  });

  describe('disconnect', () => {
    it('should close WebSocket and update status', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      // Connect first
      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      expect(getByTestId('status').children[0]).toBe('connected');

      // Now disconnect
      act(() => {
        fireEvent.press(getByTestId('disconnect-btn'));
      });

      expect(MockWebSocket.instances[0].close).toHaveBeenCalled();
      expect(getByTestId('status').children[0]).toBe('disconnected');
    });

    it('should clear streaming message on disconnect', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      // Connect and receive a message
      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'stream',
          content: 'Hello',
        });
      });

      expect(getByTestId('streaming-content').children[0]).toBe('Hello');

      // Disconnect
      act(() => {
        fireEvent.press(getByTestId('disconnect-btn'));
      });

      expect(getByTestId('streaming-content').children[0]).toBe('no-content');
    });
  });

  describe('message handling', () => {
    it('should accumulate stream content', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      // Send first chunk
      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'stream',
          content: 'Hello ',
        });
      });

      expect(getByTestId('streaming-content').children[0]).toBe('Hello ');

      // Send second chunk
      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'stream',
          content: 'World',
        });
      });

      expect(getByTestId('streaming-content').children[0]).toBe('Hello World');
    });

    it('should mark message as complete on complete event', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'stream',
          content: 'Test message',
        });
      });

      expect(getByTestId('streaming-complete').children[0]).toBe('not-complete');

      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'complete',
        });
      });

      expect(getByTestId('streaming-complete').children[0]).toBe('complete');
    });

    it('should set error status on error message', () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'error',
          message: 'Something went wrong',
        });
      });

      expect(getByTestId('status').children[0]).toBe('error');
      consoleSpy.mockRestore();
    });

    it('should handle malformed JSON gracefully', () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      // Send invalid JSON
      act(() => {
        if (MockWebSocket.instances[0].onmessage) {
          MockWebSocket.instances[0].onmessage({ data: 'not valid json' });
        }
      });

      // Should not crash - status remains connected
      expect(getByTestId('status').children[0]).toBe('connected');
      consoleSpy.mockRestore();
    });
  });

  describe('sendMessage', () => {
    it('should send message when connected', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      act(() => {
        fireEvent.press(getByTestId('send-btn'));
      });

      expect(MockWebSocket.instances[0].send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'message', content: 'Hello' })
      );
    });

    it('should clear previous streaming message when sending', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      // Receive a message first
      act(() => {
        MockWebSocket.instances[0].simulateMessage({
          type: 'stream',
          content: 'Previous content',
        });
      });

      expect(getByTestId('streaming-content').children[0]).toBe('Previous content');

      // Send a new message
      act(() => {
        fireEvent.press(getByTestId('send-btn'));
      });

      expect(getByTestId('streaming-content').children[0]).toBe('no-content');
    });

    it('should log error when sending while not connected', () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      // Try to send without connecting
      act(() => {
        fireEvent.press(getByTestId('send-btn'));
      });

      expect(consoleSpy).toHaveBeenCalledWith('WebSocket is not connected');
      consoleSpy.mockRestore();
    });
  });

  describe('error handling', () => {
    it('should set error status on WebSocket error', () => {
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateError(new Event('error'));
      });

      expect(getByTestId('status').children[0]).toBe('error');
      consoleSpy.mockRestore();
    });

    it('should set disconnected status on WebSocket close', () => {
      const { getByTestId } = render(
        <WebSocketProvider>
          <TestWebSocketConsumer />
        </WebSocketProvider>
      );

      act(() => {
        fireEvent.press(getByTestId('connect-btn'));
      });

      act(() => {
        MockWebSocket.instances[0].simulateOpen();
      });

      expect(getByTestId('status').children[0]).toBe('connected');

      // Simulate unexpected close
      act(() => {
        if (MockWebSocket.instances[0].onclose) {
          MockWebSocket.instances[0].onclose();
        }
      });

      expect(getByTestId('status').children[0]).toBe('disconnected');
    });
  });
});
