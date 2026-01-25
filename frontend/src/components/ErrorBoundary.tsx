// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Error boundary component to catch and handle React render errors gracefully
// ABOUTME: Prevents entire app crashes when a component throws during rendering

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, RefreshCw, Home } from 'lucide-react';

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Custom fallback UI to render when an error occurs */
  fallback?: ReactNode;
  /** Called when an error is caught */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  /** Whether to show a "Go Home" button (useful for nested boundaries) */
  showHomeButton?: boolean;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

/**
 * Error Boundary component that catches JavaScript errors anywhere in the child
 * component tree, logs those errors, and displays a fallback UI.
 *
 * Note: Error boundaries do NOT catch errors for:
 * - Event handlers (use try/catch)
 * - Asynchronous code (e.g., setTimeout callbacks)
 * - Server-side rendering
 * - Errors thrown in the error boundary itself
 *
 * @example
 * // Wrap entire app
 * <ErrorBoundary>
 *   <App />
 * </ErrorBoundary>
 *
 * @example
 * // Wrap specific feature with custom error handler
 * <ErrorBoundary
 *   onError={(error) => logErrorToService(error)}
 *   fallback={<CustomErrorFallback />}
 * >
 *   <RiskyFeature />
 * </ErrorBoundary>
 */
class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    // Update state so the next render will show the fallback UI
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log the error to console
    console.error('ErrorBoundary caught an error:', error);
    console.error('Component stack:', errorInfo.componentStack);

    // Update state with error info
    this.setState({ errorInfo });

    // Call custom error handler if provided
    if (this.props.onError) {
      this.props.onError(error, errorInfo);
    }
  }

  handleRetry = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  handleGoHome = (): void => {
    // Clear error state and navigate to home
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
    window.location.href = '/';
  };

  render(): ReactNode {
    const { hasError, error, errorInfo } = this.state;
    const { children, fallback, showHomeButton = true } = this.props;

    if (hasError) {
      // Return custom fallback if provided
      if (fallback) {
        return fallback;
      }

      // Default error UI
      return (
        <div className="min-h-screen bg-pierre-dark flex items-center justify-center px-4">
          <div className="max-w-lg w-full">
            {/* Error Card */}
            <div className="bg-pierre-slate rounded-xl shadow-xl overflow-hidden border border-pierre-gray-800">
              {/* Header with gradient */}
              <div className="h-1.5 w-full bg-gradient-to-r from-red-500 via-orange-500 to-red-500" />

              <div className="px-8 py-10">
                {/* Error Icon */}
                <div className="flex justify-center mb-6">
                  <div className="p-4 bg-red-500/10 rounded-full">
                    <AlertTriangle className="w-12 h-12 text-red-400" />
                  </div>
                </div>

                {/* Error Title */}
                <h1 className="text-2xl font-bold text-white text-center mb-3">
                  Something went wrong
                </h1>

                {/* Error Description */}
                <p className="text-pierre-gray-400 text-center mb-6">
                  An unexpected error occurred. We apologize for the inconvenience.
                </p>

                {/* Error Details (collapsible in production) */}
                {error && (
                  <details className="mb-6 bg-pierre-gray-900/50 rounded-lg overflow-hidden">
                    <summary className="px-4 py-3 text-sm text-pierre-gray-400 cursor-pointer hover:bg-pierre-gray-900/70 transition-colors">
                      Technical Details
                    </summary>
                    <div className="px-4 py-3 border-t border-pierre-gray-800">
                      <p className="text-sm font-mono text-red-400 mb-2">
                        {error.name}: {error.message}
                      </p>
                      {errorInfo?.componentStack && (
                        <pre className="text-xs text-pierre-gray-500 overflow-x-auto max-h-32 scrollbar-thin">
                          {errorInfo.componentStack}
                        </pre>
                      )}
                    </div>
                  </details>
                )}

                {/* Action Buttons */}
                <div className="flex flex-col sm:flex-row gap-3">
                  <button
                    onClick={this.handleRetry}
                    className="flex-1 flex items-center justify-center gap-2 px-4 py-3 bg-pierre-violet hover:bg-pierre-violet/90 text-white font-medium rounded-lg transition-colors"
                  >
                    <RefreshCw className="w-4 h-4" />
                    Try Again
                  </button>

                  {showHomeButton && (
                    <button
                      onClick={this.handleGoHome}
                      className="flex-1 flex items-center justify-center gap-2 px-4 py-3 bg-pierre-gray-700 hover:bg-pierre-gray-600 text-white font-medium rounded-lg transition-colors"
                    >
                      <Home className="w-4 h-4" />
                      Go Home
                    </button>
                  )}
                </div>
              </div>

              {/* Footer */}
              <div className="px-8 py-4 bg-pierre-gray-900/30 border-t border-pierre-gray-800">
                <p className="text-xs text-pierre-gray-500 text-center">
                  If this problem persists, please contact support or try refreshing the page.
                </p>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return children;
  }
}

export default ErrorBoundary;
