// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React, { useState, useEffect, useRef } from 'react';
import { useAuth } from '../hooks/useAuth';
import { signInWithGoogle, subscribeToAuthState, isFirebaseEnabled } from '../firebase/firebase';
import { Button, Input } from './ui';

// Pierre holistic node logo SVG inline for the login page
function PierreLogo() {
  return (
    <svg width="80" height="80" viewBox="0 0 120 120" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="pg" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#8B5CF6' }} />
          <stop offset="100%" style={{ stopColor: '#22D3EE' }} />
        </linearGradient>
        <linearGradient id="ag" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#4ADE80' }} />
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

interface LoginProps {
  onNavigateToRegister?: () => void;
}

export default function Login({ onNavigateToRegister }: LoginProps) {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [isGoogleLoading, setIsGoogleLoading] = useState(false);
  const { login, loginWithFirebase } = useAuth();
  const processingAuthRef = useRef(false);
  // Track if the user manually initiated sign-in (vs auto-login from cached Firebase state)
  const userInitiatedSignInRef = useRef(false);

  // Listen for Firebase auth state changes (handles redirect result)
  useEffect(() => {
    const unsubscribe = subscribeToAuthState(async (user) => {
      if (user && !processingAuthRef.current) {
        processingAuthRef.current = true;
        setIsGoogleLoading(true);
        // Only clear error if this is a user-initiated action
        if (userInitiatedSignInRef.current) {
          setError('');
        }

        try {
          const idToken = await user.getIdToken();
          await loginWithFirebase(idToken);
        } catch (err: unknown) {
          // Only show error if user manually initiated the sign-in
          // Auto-login from cached Firebase state should fail silently
          if (userInitiatedSignInRef.current) {
            const apiError = err as { response?: { data?: { error?: string } } };
            if (apiError.response?.data?.error) {
              setError(apiError.response.data.error);
            } else {
              const firebaseError = err as { message?: string };
              setError(firebaseError.message || 'Google sign-in failed');
            }
          }
          processingAuthRef.current = false;
          userInitiatedSignInRef.current = false;
        } finally {
          setIsGoogleLoading(false);
        }
      }
    });

    return () => unsubscribe();
  }, [loginWithFirebase]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError('');

    try {
      await login(email, password);
    } catch (err: unknown) {
      const apiError = err as { response?: { data?: { error?: string } } };
      const errorMsg = apiError.response?.data?.error || 'Login failed';
      // Map technical errors to user-friendly messages
      if (errorMsg === 'invalid_grant' || errorMsg.includes('Invalid') || errorMsg.includes('credentials')) {
        setError('Invalid email or password');
      } else {
        setError(errorMsg);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleGoogleSignIn = async () => {
    setIsGoogleLoading(true);
    setError('');
    // Mark this as a user-initiated sign-in so errors will be displayed
    userInitiatedSignInRef.current = true;

    try {
      // Initiate Google sign-in redirect (page will redirect to Google)
      await signInWithGoogle();
      // Note: signInWithRedirect will redirect the page, so code after this won't execute
    } catch (err: unknown) {
      const firebaseError = err as { code?: string; message?: string };

      if (firebaseError.code === 'auth/network-request-failed') {
        setError('Network error. Please check your connection.');
      } else {
        setError(firebaseError.message || 'Google sign-in failed');
      }
      setIsGoogleLoading(false);
      userInitiatedSignInRef.current = false;
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8 bg-pierre-dark">
      <div className="max-w-md w-full">
        {/* Card with glassmorphism and gradient accent bar */}
        <div
          className="rounded-xl border border-white/10 overflow-hidden relative"
          style={{
            background: 'rgba(30, 30, 46, 0.8)',
            backdropFilter: 'blur(12px)',
          }}
        >
          {/* Gradient accent bar at top */}
          <div className="h-1 w-full bg-gradient-pierre-horizontal" />

          <div className="px-8 py-10 space-y-6">
            {/* Logo and brand */}
            <div className="flex flex-col items-center">
              <PierreLogo />
              <h1 className="mt-4 text-xl font-bold text-white tracking-tight">
                Pierre Fitness Platform
              </h1>
              <p className="mt-1 text-sm text-zinc-400">
                Sign in to your account
              </p>
            </div>

            {/* Login form */}
            <form className="space-y-5" onSubmit={handleSubmit}>
              {error && (
                <div
                  role="alert"
                  aria-live="polite"
                  className="bg-red-500/10 border border-red-500/30 text-red-400 px-4 py-3 rounded-lg text-sm"
                >
                  {error}
                </div>
              )}

              <div className="space-y-4">
                <Input
                  id="email"
                  name="email"
                  type="email"
                  label="Email address"
                  autoComplete="email"
                  required
                  placeholder="Enter your email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  variant="dark"
                />
                <Input
                  id="password"
                  name="password"
                  type={showPassword ? 'text' : 'password'}
                  label="Password"
                  autoComplete="current-password"
                  required
                  placeholder="Enter your password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  variant="dark"
                  rightIcon={
                    <button
                      type="button"
                      aria-label={showPassword ? 'Hide password' : 'Show password'}
                      className="text-zinc-400 hover:text-white transition-colors"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L3 3m6.878 6.878L21 21" />
                        </svg>
                      ) : (
                        <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      )}
                    </button>
                  }
                />
              </div>

              <Button
                type="submit"
                variant="gradient"
                loading={isLoading}
                className="w-full shadow-glow-sm"
              >
                {isLoading ? 'Signing in...' : 'Sign in'}
              </Button>

              {/* Google Sign-In - only show when Firebase is configured */}
              {isFirebaseEnabled() && (
                <>
                  {/* Divider */}
                  <div className="relative my-4">
                    <div className="absolute inset-0 flex items-center">
                      <div className="w-full border-t border-white/10" />
                    </div>
                    <div className="relative flex justify-center text-sm">
                      <span className="px-2 bg-pierre-slate text-zinc-500">or continue with</span>
                    </div>
                  </div>

                  {/* Google Sign-In Button */}
                  <button
                    type="button"
                    onClick={handleGoogleSignIn}
                    disabled={isGoogleLoading}
                    className="w-full flex items-center justify-center gap-3 px-4 py-2.5 border border-white/10 rounded-lg bg-white/5 hover:bg-white/10 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {isGoogleLoading ? (
                      <div className="pierre-spinner w-5 h-5 border-zinc-400 border-t-transparent"></div>
                    ) : (
                      <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
                        <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
                        <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/>
                        <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/>
                      </svg>
                    )}
                    <span className="text-zinc-300 font-medium">
                      {isGoogleLoading ? 'Signing in...' : 'Continue with Google'}
                    </span>
                  </button>
                </>
              )}

              {/* Link to register */}
              {onNavigateToRegister && (
                <div className="text-center pt-2">
                  <button
                    type="button"
                    onClick={onNavigateToRegister}
                    className="text-sm text-pierre-violet-light hover:text-pierre-cyan-light font-medium transition-colors"
                  >
                    Don&apos;t have an account? Create one
                  </button>
                </div>
              )}
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}