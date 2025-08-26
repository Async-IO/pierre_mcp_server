import React, { useState, useEffect } from 'react';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { SetupStatusResponse } from '../types/api';

export default function Login() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [setupStatus, setSetupStatus] = useState<SetupStatusResponse | null>(null);
  const [isCheckingSetup, setIsCheckingSetup] = useState(true);
  const { login } = useAuth();

  useEffect(() => {
    const checkSetupStatus = async () => {
      try {
        const status = await apiService.getSetupStatus();
        setSetupStatus(status);
      } catch (error) {
        console.error('Failed to check setup status:', error);
        // Default to showing setup instructions if we can't check
        setSetupStatus({
          needs_setup: true,
          admin_user_exists: false,
          message: 'Unable to verify setup status. Please ensure admin user is created.',
        });
      } finally {
        setIsCheckingSetup(false);
      }
    };

    checkSetupStatus();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError('');

    try {
      await login(email, password);
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: string } } };
      setError(error.response?.data?.error || 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        <div>
          <h2 className="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Pierre MCP Admin
          </h2>
          <p className="mt-2 text-center text-sm text-gray-600">
            Sign in to manage admin tokens and API keys
          </p>
        </div>
        <form className="mt-8 space-y-6" onSubmit={handleSubmit}>
          {error && (
            <div className="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded">
              {error}
            </div>
          )}
          <div className="rounded-md shadow-sm space-y-4">
            <div>
              <label htmlFor="email" className="block text-sm font-medium text-gray-700">
                Email address
              </label>
              <input
                id="email"
                name="email"
                type="email"
                required
                className="input-field"
                placeholder="Enter your email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
            </div>
            <div>
              <label htmlFor="password" className="block text-sm font-medium text-gray-700">
                Password
              </label>
              <div className="relative">
                <input
                  id="password"
                  name="password"
                  type={showPassword ? 'text' : 'password'}
                  required
                  className="input-field pr-12"
                  placeholder="Enter your password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
                <button
                  type="button"
                  className="absolute inset-y-0 right-0 flex items-center pr-3"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? (
                    <svg
                      className="h-5 w-5 text-pierre-gray-400 hover:text-pierre-gray-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L3 3m6.878 6.878L21 21"
                      />
                    </svg>
                  ) : (
                    <svg
                      className="h-5 w-5 text-pierre-gray-400 hover:text-pierre-gray-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                      />
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                      />
                    </svg>
                  )}
                </button>
              </div>
            </div>
          </div>

          <div>
            <button
              type="submit"
              disabled={isLoading}
              className="btn-primary w-full justify-center py-3 px-4 text-sm font-medium disabled:opacity-50"
            >
              {isLoading ? 'Signing in...' : 'Sign in'}
            </button>
          </div>

          {/* Conditional setup instructions */}
          {isCheckingSetup && (
            <div className="text-center">
              <div className="bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg p-4">
                <p className="text-sm text-pierre-gray-600">Checking setup status...</p>
              </div>
            </div>
          )}

          {!isCheckingSetup && setupStatus?.needs_setup && (
            <div className="text-center">
              <div className="bg-pierre-blue-50 border border-pierre-blue-200 rounded-lg p-4">
                <p className="text-sm text-pierre-gray-700 font-medium mb-2">
                  💡 First Time Setup Required
                </p>
                <p className="text-xs text-pierre-gray-500">
                  {setupStatus.message || 'Use the server API to create your admin credentials:'}<br/>
                  <code className="text-pierre-gray-700">POST /admin/setup with email/password/display_name</code>
                </p>
              </div>
            </div>
          )}

          {!isCheckingSetup && setupStatus && !setupStatus.needs_setup && (
            <div className="text-center">
              <div className="bg-pierre-green-50 border border-pierre-green-200 rounded-lg p-4">
                <p className="text-sm text-pierre-gray-700 font-medium mb-2">
                  ✅ Admin Setup Complete
                </p>
                <p className="text-xs text-pierre-gray-500">
                  Admin user is configured. You can login with your credentials.
                </p>
              </div>
            </div>
          )}
        </form>
      </div>
    </div>
  );
}