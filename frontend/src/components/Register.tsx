// ABOUTME: User registration page for new account creation
// ABOUTME: Matches Login.tsx design with Pierre brand aesthetic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import React, { useState } from 'react';
import { authApi } from '../services/api';
import { Button, Input } from './ui';

// Pierre holistic node logo SVG (shared with Login)
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

interface RegisterProps {
  onNavigateToLogin: () => void;
  onRegistrationSuccess: (message: string) => void;
}

export default function Register({ onNavigateToLogin, onRegistrationSuccess }: RegisterProps) {
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    // Validate passwords match
    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    // Validate password strength
    if (password.length < 8) {
      setError('Password must be at least 8 characters');
      return;
    }

    setIsLoading(true);

    try {
      const response = await authApi.register({ email, password, display_name: displayName || undefined });
      onRegistrationSuccess(response.message);
    } catch (err: unknown) {
      const apiError = err as { response?: { data?: { message?: string; error?: string } } };
      setError(apiError.response?.data?.message || apiError.response?.data?.error || 'Registration failed. Please try again.');
    } finally {
      setIsLoading(false);
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
                Create Your Account
              </h1>
              <p className="mt-1 text-sm text-zinc-400">
                Join Pierre Fitness Intelligence
              </p>
            </div>

            {/* Registration form */}
            <form className="space-y-5" onSubmit={handleSubmit}>
              {error && (
                <div className="bg-red-500/10 border border-red-500/30 text-red-400 px-4 py-3 rounded-lg text-sm">
                  {error}
                </div>
              )}

              <div className="space-y-4">
                <Input
                  id="displayName"
                  name="displayName"
                  type="text"
                  label="Display Name"
                  autoComplete="name"
                  placeholder="Enter your name (optional)"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  variant="dark"
                />

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
                  autoComplete="new-password"
                  required
                  placeholder="Create a password (min 8 characters)"
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
                  }
                />

                <Input
                  id="confirmPassword"
                  name="confirmPassword"
                  type={showPassword ? 'text' : 'password'}
                  label="Confirm Password"
                  autoComplete="new-password"
                  required
                  placeholder="Confirm your password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  variant="dark"
                />
              </div>

              <Button
                type="submit"
                variant="gradient"
                loading={isLoading}
                className="w-full shadow-glow-sm"
              >
                {isLoading ? 'Creating account...' : 'Create Account'}
              </Button>

              {/* Link to login */}
              <div className="text-center">
                <button
                  type="button"
                  onClick={onNavigateToLogin}
                  className="text-sm text-pierre-violet-light hover:text-pierre-cyan-light font-medium transition-colors"
                >
                  Already have an account? Sign in
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}
