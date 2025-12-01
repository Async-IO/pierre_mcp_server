import React, { useState, useEffect } from 'react';
import { useAuth } from '../hooks/useAuth';
import { apiService } from '../services/api';
import type { SetupStatusResponse } from '../types/api';

// Pierre holistic node logo SVG inline for the login page
function PierreLogo() {
  return (
    <svg width="80" height="80" viewBox="0 0 120 120" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="pg" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#7C3AED' }} />
          <stop offset="100%" style={{ stopColor: '#06B6D4' }} />
        </linearGradient>
        <linearGradient id="ag" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#10B981' }} />
          <stop offset="100%" style={{ stopColor: '#059669' }} />
        </linearGradient>
        <linearGradient id="ng" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#F59E0B' }} />
          <stop offset="100%" style={{ stopColor: '#D97706' }} />
        </linearGradient>
        <linearGradient id="rg" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#6366F1' }} />
          <stop offset="100%" style={{ stopColor: '#4F46E5' }} />
        </linearGradient>
      </defs>
      <g strokeWidth="2" opacity="0.5" strokeLinecap="round">
        <line x1="40" y1="30" x2="52" y2="42" stroke="url(#ag)" />
        <line x1="52" y1="42" x2="70" y2="35" stroke="url(#ag)" />
        <line x1="52" y1="42" x2="48" y2="55" stroke="url(#pg)" />
        <line x1="48" y1="55" x2="75" y2="52" stroke="url(#ng)" />
        <line x1="48" y1="55" x2="55" y2="72" stroke="url(#pg)" />
        <line x1="55" y1="72" x2="35" y2="85" stroke="url(#rg)" />
        <line x1="55" y1="72" x2="72" y2="82" stroke="url(#rg)" />
      </g>
      <circle cx="40" cy="30" r="7" fill="url(#ag)" />
      <circle cx="52" cy="42" r="5" fill="url(#ag)" />
      <circle cx="70" cy="35" r="3.5" fill="url(#ag)" />
      <circle cx="48" cy="55" r="6" fill="url(#pg)" />
      <circle cx="48" cy="55" r="3" fill="#fff" opacity="0.9" />
      <circle cx="75" cy="52" r="4.5" fill="url(#ng)" />
      <circle cx="88" cy="60" r="3.5" fill="url(#ng)" />
      <circle cx="55" cy="72" r="5" fill="url(#rg)" />
      <circle cx="35" cy="85" r="4" fill="url(#rg)" />
      <circle cx="72" cy="82" r="4" fill="url(#rg)" />
    </svg>
  );
}

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
      } catch (err) {
        console.error('Failed to check setup status:', err);
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
      const apiError = err as { response?: { data?: { error?: string } } };
      setError(apiError.response?.data?.error || 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8 bg-pierre-gray-50">
      <div className="max-w-md w-full">
        {/* Card with gradient accent bar */}
        <div className="bg-white rounded-xl shadow-lg overflow-hidden relative">
          {/* Gradient accent bar at top */}
          <div className="h-1 w-full bg-gradient-pierre-horizontal" />

          <div className="px-8 py-10 space-y-6">
            {/* Logo and brand */}
            <div className="flex flex-col items-center">
              <PierreLogo />
              <h1 className="mt-4 text-xl font-bold text-pierre-gray-900 tracking-tight">
                Pierre Fitness Platform
              </h1>
              <p className="mt-1 text-sm text-pierre-gray-500">
                Sign in to manage admin tokens and API keys
              </p>
            </div>

            {/* Login form */}
            <form className="space-y-5" onSubmit={handleSubmit}>
              {error && (
                <div className="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-lg text-sm">
                  {error}
                </div>
              )}

              <div className="space-y-4">
                <div>
                  <label htmlFor="email" className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Email address
                  </label>
                  <input
                    id="email"
                    name="email"
                    type="email"
                    autoComplete="email"
                    required
                    className="w-full px-4 py-2.5 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent outline-none transition-all"
                    placeholder="Enter your email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                  />
                </div>
                <div>
                  <label htmlFor="password" className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Password
                  </label>
                  <div className="relative">
                    <input
                      id="password"
                      name="password"
                      type={showPassword ? 'text' : 'password'}
                      autoComplete="current-password"
                      required
                      className="w-full px-4 py-2.5 pr-12 border border-pierre-gray-300 rounded-lg focus:ring-2 focus:ring-pierre-violet focus:border-transparent outline-none transition-all"
                      placeholder="Enter your password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                    />
                    <button
                      type="button"
                      aria-label={showPassword ? 'Hide password' : 'Show password'}
                      className="absolute inset-y-0 right-0 flex items-center pr-3 text-pierre-gray-400 hover:text-pierre-gray-600"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L3 3m6.878 6.878L21 21" />
                        </svg>
                      ) : (
                        <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      )}
                    </button>
                  </div>
                </div>
              </div>

              <button
                type="submit"
                disabled={isLoading}
                className="w-full py-3 px-4 text-white font-medium rounded-lg bg-gradient-pierre-horizontal hover:opacity-90 disabled:opacity-50 transition-all shadow-md hover:shadow-lg"
              >
                {isLoading ? 'Signing in...' : 'Sign in'}
              </button>

              {/* Setup status indicator */}
              {isCheckingSetup && (
                <div className="bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg p-3 text-center">
                  <p className="text-sm text-pierre-gray-500">Checking setup status...</p>
                </div>
              )}

              {!isCheckingSetup && setupStatus?.needs_setup && (
                <div className="bg-pierre-nutrition-light/20 border border-pierre-nutrition rounded-lg p-3 text-center">
                  <p className="text-sm text-pierre-gray-700 font-medium">Setup Required</p>
                  <p className="text-xs text-pierre-gray-500 mt-1">
                    {setupStatus.message || 'Create admin via POST /admin/setup'}
                  </p>
                </div>
              )}

              {!isCheckingSetup && setupStatus && !setupStatus.needs_setup && (
                <div className="bg-pierre-activity-light/20 border border-pierre-activity rounded-lg p-3 text-center">
                  <p className="text-sm text-pierre-gray-700 font-medium">Ready to Login</p>
                  <p className="text-xs text-pierre-gray-500 mt-1">Admin user configured</p>
                </div>
              )}
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}