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
import { AuthProvider } from './src/contexts/AuthContext';
import { QueryProvider } from './src/providers/QueryProvider';
import { WebSocketProvider } from './src/contexts/WebSocketContext';
import { RootNavigator } from './src/navigation/RootNavigator';

export default function App() {
  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <SafeAreaProvider>
        <AuthProvider>
          <QueryProvider>
            <WebSocketProvider>
              <StatusBar style="light" />
              <RootNavigator />
              <Toast config={toastConfig} />
            </WebSocketProvider>
          </QueryProvider>
        </AuthProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}
