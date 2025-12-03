// ABOUTME: Page shown to users whose accounts are pending admin approval
// ABOUTME: Displays status message and allows logout while waiting for approval
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useAuth } from '../hooks/useAuth';
import { Button, Card, Badge } from './ui';

// Clock icon for pending status
function ClockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}

// Pierre holistic node logo SVG
function PierreLogo() {
  return (
    <svg width="64" height="64" viewBox="0 0 120 120" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="pg-pending" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#7C3AED' }} />
          <stop offset="100%" style={{ stopColor: '#06B6D4' }} />
        </linearGradient>
        <linearGradient id="ag-pending" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#10B981' }} />
          <stop offset="100%" style={{ stopColor: '#059669' }} />
        </linearGradient>
        <linearGradient id="ng-pending" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#F59E0B' }} />
          <stop offset="100%" style={{ stopColor: '#D97706' }} />
        </linearGradient>
        <linearGradient id="rg-pending" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style={{ stopColor: '#6366F1' }} />
          <stop offset="100%" style={{ stopColor: '#4F46E5' }} />
        </linearGradient>
      </defs>
      <g strokeWidth="2" opacity="0.5" strokeLinecap="round">
        <line x1="40" y1="30" x2="52" y2="42" stroke="url(#ag-pending)" />
        <line x1="52" y1="42" x2="70" y2="35" stroke="url(#ag-pending)" />
        <line x1="52" y1="42" x2="48" y2="55" stroke="url(#pg-pending)" />
        <line x1="48" y1="55" x2="75" y2="52" stroke="url(#ng-pending)" />
        <line x1="48" y1="55" x2="55" y2="72" stroke="url(#pg-pending)" />
        <line x1="55" y1="72" x2="35" y2="85" stroke="url(#rg-pending)" />
        <line x1="55" y1="72" x2="72" y2="82" stroke="url(#rg-pending)" />
      </g>
      <circle cx="40" cy="30" r="7" fill="url(#ag-pending)" />
      <circle cx="52" cy="42" r="5" fill="url(#ag-pending)" />
      <circle cx="70" cy="35" r="3.5" fill="url(#ag-pending)" />
      <circle cx="48" cy="55" r="6" fill="url(#pg-pending)" />
      <circle cx="48" cy="55" r="3" fill="#fff" opacity="0.9" />
      <circle cx="75" cy="52" r="4.5" fill="url(#ng-pending)" />
      <circle cx="88" cy="60" r="3.5" fill="url(#ng-pending)" />
      <circle cx="55" cy="72" r="5" fill="url(#rg-pending)" />
      <circle cx="35" cy="85" r="4" fill="url(#rg-pending)" />
      <circle cx="72" cy="82" r="4" fill="url(#rg-pending)" />
    </svg>
  );
}

export default function PendingApproval() {
  const { user, logout } = useAuth();

  return (
    <div className="min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8 bg-pierre-gray-50">
      <div className="max-w-md w-full">
        <Card className="overflow-hidden">
          {/* Gradient accent bar at top */}
          <div className="h-1 w-full bg-gradient-pierre-horizontal" />

          <div className="px-8 py-10">
            {/* Logo and icon */}
            <div className="flex flex-col items-center text-center">
              <PierreLogo />

              <div className="mt-6 mb-4">
                <ClockIcon className="w-16 h-16 text-pierre-nutrition mx-auto" />
              </div>

              <h1 className="text-xl font-bold text-pierre-gray-900">
                Account Pending Approval
              </h1>

              <p className="mt-3 text-sm text-pierre-gray-600 max-w-sm">
                Your account has been created successfully and is awaiting approval
                by an administrator. You&apos;ll receive an email notification once
                your account is approved.
              </p>
            </div>

            {/* Status card */}
            <div className="mt-8 bg-pierre-gray-50 rounded-lg p-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-pierre-gray-700">Status</span>
                <Badge variant="warning">Pending</Badge>
              </div>

              {user?.email && (
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-pierre-gray-700">Email</span>
                  <span className="text-sm text-pierre-gray-600">{user.email}</span>
                </div>
              )}

              {user?.display_name && (
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-pierre-gray-700">Name</span>
                  <span className="text-sm text-pierre-gray-600">{user.display_name}</span>
                </div>
              )}
            </div>

            {/* What happens next */}
            <div className="mt-6">
              <h2 className="text-sm font-semibold text-pierre-gray-900 mb-3">
                What happens next?
              </h2>
              <ul className="text-sm text-pierre-gray-600 space-y-2">
                <li className="flex items-start gap-2">
                  <span className="text-pierre-activity mt-0.5">•</span>
                  <span>An administrator will review your registration</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-pierre-activity mt-0.5">•</span>
                  <span>You&apos;ll receive an email when approved</span>
                </li>
                <li className="flex items-start gap-2">
                  <span className="text-pierre-activity mt-0.5">•</span>
                  <span>Once approved, you can access Pierre&apos;s fitness intelligence</span>
                </li>
              </ul>
            </div>

            {/* Sign out button */}
            <div className="mt-8">
              <Button
                variant="secondary"
                onClick={logout}
                className="w-full"
              >
                Sign Out
              </Button>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}
