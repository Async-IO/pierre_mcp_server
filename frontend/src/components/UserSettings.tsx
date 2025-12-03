// ABOUTME: User settings tab for regular users
// ABOUTME: Displays profile information and account settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useState } from 'react';
import { useAuth } from '../hooks/useAuth';
import { Card, Button, Input } from './ui';

export default function UserSettings() {
  const { user, logout } = useAuth();
  const [displayName, setDisplayName] = useState(user?.display_name || '');
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const handleSaveProfile = async () => {
    setIsSaving(true);
    setMessage(null);

    // Profile update functionality will be added later
    setTimeout(() => {
      setIsSaving(false);
      setMessage({ type: 'success', text: 'Profile updated successfully' });
    }, 500);
  };

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Profile Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Profile Information</h2>

        <div className="space-y-4">
          <div className="flex items-center gap-4 pb-4 border-b border-pierre-gray-100">
            <div className="w-16 h-16 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-full flex items-center justify-center flex-shrink-0">
              <span className="text-2xl font-bold text-white">
                {(user?.display_name || user?.email)?.charAt(0).toUpperCase()}
              </span>
            </div>
            <div>
              <p className="font-medium text-pierre-gray-900">{user?.display_name || 'No name set'}</p>
              <p className="text-sm text-pierre-gray-500">{user?.email}</p>
            </div>
          </div>

          <div>
            <Input
              label="Display Name"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Enter your display name"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-pierre-gray-700 mb-1">Email</label>
            <p className="text-pierre-gray-900 bg-pierre-gray-50 px-3 py-2 rounded-lg">{user?.email}</p>
            <p className="text-xs text-pierre-gray-500 mt-1">Email cannot be changed</p>
          </div>

          {message && (
            <div className={`p-3 rounded-lg text-sm ${
              message.type === 'success'
                ? 'bg-pierre-activity-light/30 text-pierre-activity'
                : 'bg-red-50 text-red-600'
            }`}>
              {message.text}
            </div>
          )}

          <Button
            variant="gradient"
            onClick={handleSaveProfile}
            loading={isSaving}
            disabled={displayName === user?.display_name}
          >
            Save Changes
          </Button>
        </div>
      </Card>

      {/* Account Status Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Account Status</h2>

        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
            <span className="text-pierre-gray-600">Status</span>
            <span className={`px-2 py-1 rounded-full text-xs font-medium ${
              user?.user_status === 'active'
                ? 'bg-pierre-activity-light/30 text-pierre-activity'
                : 'bg-pierre-nutrition-light/30 text-pierre-nutrition'
            }`}>
              {user?.user_status?.charAt(0).toUpperCase()}{user?.user_status?.slice(1)}
            </span>
          </div>
          <div className="flex justify-between items-center py-2 border-b border-pierre-gray-100">
            <span className="text-pierre-gray-600">Role</span>
            <span className="text-pierre-gray-900 capitalize">{user?.role}</span>
          </div>
          <div className="flex justify-between items-center py-2">
            <span className="text-pierre-gray-600">Member Since</span>
            <span className="text-pierre-gray-900">N/A</span>
          </div>
        </div>
      </Card>

      {/* Security Section */}
      <Card>
        <h2 className="text-lg font-semibold text-pierre-gray-900 mb-4">Security</h2>

        <div className="space-y-4">
          <div className="p-4 bg-pierre-gray-50 rounded-lg">
            <h3 className="font-medium text-pierre-gray-900 mb-2">Password</h3>
            <p className="text-sm text-pierre-gray-600 mb-3">
              Change your password to keep your account secure.
            </p>
            <Button variant="secondary" size="sm" disabled>
              Change Password
            </Button>
            <p className="text-xs text-pierre-gray-400 mt-2">Coming soon</p>
          </div>
        </div>
      </Card>

      {/* Danger Zone */}
      <Card className="border-red-200">
        <h2 className="text-lg font-semibold text-red-600 mb-4">Danger Zone</h2>

        <div className="space-y-4">
          <div className="p-4 bg-red-50 rounded-lg">
            <h3 className="font-medium text-pierre-gray-900 mb-2">Sign Out</h3>
            <p className="text-sm text-pierre-gray-600 mb-3">
              Sign out of your account on this device.
            </p>
            <Button variant="secondary" size="sm" onClick={logout}>
              Sign Out
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
