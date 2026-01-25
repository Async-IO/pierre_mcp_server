// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState, useEffect } from 'react';
import { QueryClient, QueryClientProvider, useQueryClient } from '@tanstack/react-query';
import Login from './components/Login';
import Register from './components/Register';
import PendingApproval from './components/PendingApproval';
import Dashboard from './components/Dashboard';
import ImpersonationBanner from './components/ImpersonationBanner';
import ConnectionBanner from './components/ConnectionBanner';
import OAuthCallback from './components/OAuthCallback';
import ErrorBoundary from './components/ErrorBoundary';
import { AuthProvider } from './contexts/AuthContext';
import { WebSocketProvider } from './contexts/WebSocketProvider';
import { useAuth } from './hooks/useAuth';
import './App.css';

const queryClient = new QueryClient();

/**
 * Check if the current URL has OAuth callback parameters
 */
function getOAuthCallbackParams(): { provider: string; success: boolean; error?: string } | null {
  const urlParams = new URLSearchParams(window.location.search);
  const provider = urlParams.get('provider');
  const success = urlParams.get('success');

  if (provider && success !== null) {
    return {
      provider,
      success: success === 'true',
      error: urlParams.get('error') || undefined,
    };
  }
  return null;
}

type AuthView = 'login' | 'register';

function AppContent() {
  const { user, isAuthenticated, isLoading } = useAuth();
  const [authView, setAuthView] = useState<AuthView>('login');
  const [registrationMessage, setRegistrationMessage] = useState<string | null>(null);
  const [oauthCallback, setOauthCallback] = useState<{ provider: string; success: boolean; error?: string } | null>(null);
  const localQueryClient = useQueryClient();

  // Check for OAuth callback params on mount
  useEffect(() => {
    const params = getOAuthCallbackParams();
    if (params) {
      setOauthCallback(params);
      // Invalidate OAuth status queries to refresh connection state
      localQueryClient.invalidateQueries({ queryKey: ['oauth-status'] });
      localQueryClient.invalidateQueries({ queryKey: ['connections'] });
    }
  }, [localQueryClient]);

  // Show OAuth callback result page
  if (oauthCallback) {
    return (
      <OAuthCallback
        provider={oauthCallback.provider}
        success={oauthCallback.success}
        error={oauthCallback.error}
        onClose={() => {
          // Dispatch event for ChatTab to detect OAuth completion
          window.postMessage({
            type: 'oauth_completed',
            provider: oauthCallback.provider,
            success: oauthCallback.success,
          }, window.location.origin);
          // Clear URL params and close the callback view
          window.history.replaceState({}, document.title, window.location.pathname);
          setOauthCallback(null);
        }}
      />
    );
  }

  if (isLoading) {
    return (
      <div className="min-h-screen bg-pierre-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-pierre-violet mx-auto"></div>
          <p className="mt-4 text-pierre-gray-600">Loading...</p>
        </div>
      </div>
    );
  }

  // Not authenticated - show login or register
  if (!isAuthenticated) {
    if (authView === 'register') {
      return (
        <Register
          onNavigateToLogin={() => {
            setAuthView('login');
            setRegistrationMessage(null);
          }}
          onRegistrationSuccess={(message) => {
            setRegistrationMessage(message);
            setAuthView('login');
          }}
        />
      );
    }

    return (
      <div className="min-h-screen bg-pierre-gray-50">
        {registrationMessage && (
          <div className="fixed top-4 left-1/2 transform -translate-x-1/2 z-50 max-w-md w-full px-4">
            <div className="bg-pierre-activity-light border border-pierre-activity text-pierre-gray-900 px-4 py-3 rounded-lg shadow-lg">
              <div className="flex items-center justify-between">
                <p className="text-sm font-medium">{registrationMessage}</p>
                <button
                  onClick={() => setRegistrationMessage(null)}
                  className="ml-4 text-pierre-gray-600 hover:text-pierre-gray-900"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        )}
        <Login onNavigateToRegister={() => setAuthView('register')} />
      </div>
    );
  }

  // Authenticated but pending approval
  if (user?.user_status === 'pending') {
    return <PendingApproval />;
  }

  // Authenticated but suspended
  if (user?.user_status === 'suspended') {
    return (
      <div className="min-h-screen bg-pierre-gray-50 flex items-center justify-center px-4">
        <div className="max-w-md w-full bg-white rounded-xl shadow-lg overflow-hidden">
          <div className="h-1 w-full bg-gradient-to-r from-red-500 to-red-600" />
          <div className="px-8 py-10 text-center">
            <svg className="w-16 h-16 text-red-500 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
            </svg>
            <h1 className="text-xl font-bold text-pierre-gray-900 mb-2">Account Suspended</h1>
            <p className="text-sm text-pierre-gray-600 mb-6">
              Your account has been suspended. Please contact an administrator for assistance.
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Authenticated and active - show dashboard
  return (
    <div className="min-h-screen bg-pierre-gray-50">
      <ConnectionBanner />
      <ImpersonationBanner />
      <Dashboard />
    </div>
  );
}

function App() {
  return (
    <ErrorBoundary
      onError={(error, errorInfo) => {
        // Log errors to console in development
        // In production, this could send to an error tracking service
        console.error('Application error:', error);
        console.error('Component stack:', errorInfo.componentStack);
      }}
    >
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <WebSocketProvider>
            <AppContent />
          </WebSocketProvider>
        </AuthProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

export default App;
