// ABOUTME: Main application entry point for Pierre Mobile
// ABOUTME: Sets up providers (Auth, Query, WebSocket) and navigation with dark theme

import './global.css';
import React from 'react';
import { LogBox } from 'react-native';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import Toast from 'react-native-toast-message';
import { toastConfig } from './src/config/toast';

// Ignore expected error logs in development
// These are handled with user-friendly Alert dialogs in the UI
LogBox.ignoreLogs([
  'Failed to send message:',
  'Failed to load conversations:',
  'Failed to load messages:',
  'Failed to create conversation:',
  'AxiosError',
]);
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider } from './src/contexts/AuthContext';
import { WebSocketProvider } from './src/contexts/WebSocketContext';
import { RootNavigator } from './src/navigation/RootNavigator';

// Create a query client for React Query
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 1000 * 60 * 5, // 5 minutes
    },
  },
});

export default function App() {
  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <SafeAreaProvider>
        <QueryClientProvider client={queryClient}>
          <AuthProvider>
            <WebSocketProvider>
              <StatusBar style="light" />
              <RootNavigator />
              <Toast config={toastConfig} />
            </WebSocketProvider>
          </AuthProvider>
        </QueryClientProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}
