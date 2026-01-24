// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Social settings tab for managing privacy and notification preferences
// ABOUTME: Controls discoverability, default visibility, and notification settings

import { useState, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { apiService } from '../../services/api';
import { Card, Button } from '../ui';

interface NotificationPreferences {
  friend_requests: boolean;
  insight_reactions: boolean;
  adapted_insights: boolean;
}

interface SocialSettings {
  user_id: string;
  discoverable: boolean;
  default_visibility: string;
  share_activity_types: string[];
  notifications: NotificationPreferences;
  created_at: string;
  updated_at: string;
}

interface SettingRowProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  value: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}

function SettingRow({ icon, title, description, value, onChange, disabled }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between py-4">
      <div className="flex items-start gap-3">
        <div className="w-10 h-10 rounded-lg bg-pierre-violet/20 flex items-center justify-center flex-shrink-0">
          {icon}
        </div>
        <div>
          <p className="font-medium text-white">{title}</p>
          <p className="text-sm text-zinc-500 mt-0.5">{description}</p>
        </div>
      </div>
      <button
        onClick={() => onChange(!value)}
        disabled={disabled}
        className={clsx(
          'relative w-11 h-6 rounded-full transition-colors',
          value ? 'bg-pierre-violet' : 'bg-white/20',
          disabled && 'opacity-50 cursor-not-allowed'
        )}
      >
        <div
          className={clsx(
            'absolute top-1 w-4 h-4 rounded-full bg-white transition-transform',
            value ? 'translate-x-6' : 'translate-x-1'
          )}
        />
      </button>
    </div>
  );
}

export default function SocialSettingsTab() {
  const [settings, setSettings] = useState<SocialSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);

  const loadSettings = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await apiService.getSocialSettings();
      setSettings(response.settings);
    } catch (error) {
      console.error('Failed to load social settings:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const updateSetting = <K extends keyof SocialSettings>(
    key: K,
    value: SocialSettings[K]
  ) => {
    if (!settings) return;
    setSettings({ ...settings, [key]: value });
    setHasChanges(true);
    setSaveSuccess(false);
  };

  const updateNotification = (
    key: keyof NotificationPreferences,
    value: boolean
  ) => {
    if (!settings) return;
    setSettings({
      ...settings,
      notifications: { ...settings.notifications, [key]: value },
    });
    setHasChanges(true);
    setSaveSuccess(false);
  };

  const handleSave = async () => {
    if (!settings || !hasChanges) return;

    try {
      setIsSaving(true);
      await apiService.updateSocialSettings({
        discoverable: settings.discoverable,
        default_visibility: settings.default_visibility,
        notifications: settings.notifications,
      });
      setHasChanges(false);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (error) {
      console.error('Failed to save settings:', error);
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading || !settings) {
    return (
      <div className="flex justify-center py-8">
        <div className="pierre-spinner"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-white">Social Settings</h2>
          <p className="text-sm text-zinc-400 mt-1">
            Manage your privacy and notification preferences
          </p>
        </div>
        <div className="flex items-center gap-3">
          {saveSuccess && (
            <span className="text-sm text-pierre-activity flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
              Saved
            </span>
          )}
          <Button
            variant="primary"
            onClick={handleSave}
            disabled={!hasChanges}
            loading={isSaving}
          >
            Save Changes
          </Button>
        </div>
      </div>

      {/* Privacy Section */}
      <Card variant="dark" className="!p-5">
        <h3 className="text-sm font-semibold text-zinc-400 mb-4 uppercase tracking-wide">Privacy</h3>
        <SettingRow
          icon={
            <svg className="w-5 h-5 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
          }
          title="Discoverable"
          description="Allow others to find you when searching for friends"
          value={settings.discoverable}
          onChange={(value) => updateSetting('discoverable', value)}
        />
      </Card>

      {/* Default Sharing Section */}
      <Card variant="dark" className="!p-5">
        <h3 className="text-sm font-semibold text-zinc-400 mb-4 uppercase tracking-wide">Default Sharing</h3>
        <p className="text-sm text-zinc-500 mb-4">Default visibility for new insights</p>
        <div className="flex gap-3">
          <button
            onClick={() => updateSetting('default_visibility', 'friends_only')}
            className={clsx(
              'flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg border transition-colors',
              settings.default_visibility === 'friends_only'
                ? 'bg-pierre-violet/20 border-pierre-violet text-pierre-violet-light'
                : 'bg-white/5 border-white/10 text-zinc-400 hover:bg-white/10'
            )}
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
            Friends Only
          </button>
          <button
            onClick={() => updateSetting('default_visibility', 'public')}
            className={clsx(
              'flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg border transition-colors',
              settings.default_visibility === 'public'
                ? 'bg-pierre-violet/20 border-pierre-violet text-pierre-violet-light'
                : 'bg-white/5 border-white/10 text-zinc-400 hover:bg-white/10'
            )}
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Public
          </button>
        </div>
      </Card>

      {/* Notifications Section */}
      <Card variant="dark" className="!p-5">
        <h3 className="text-sm font-semibold text-zinc-400 mb-4 uppercase tracking-wide">Notifications</h3>
        <div className="divide-y divide-white/10">
          <SettingRow
            icon={
              <svg className="w-5 h-5 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
              </svg>
            }
            title="Friend Requests"
            description="Get notified when someone sends you a friend request"
            value={settings.notifications.friend_requests}
            onChange={(value) => updateNotification('friend_requests', value)}
          />
          <SettingRow
            icon={
              <svg className="w-5 h-5 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
              </svg>
            }
            title="Reactions"
            description="Get notified when someone reacts to your insights"
            value={settings.notifications.insight_reactions}
            onChange={(value) => updateNotification('insight_reactions', value)}
          />
          <SettingRow
            icon={
              <svg className="w-5 h-5 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            }
            title="Adapted Insights"
            description="Get notified when someone adapts your shared insight"
            value={settings.notifications.adapted_insights}
            onChange={(value) => updateNotification('adapted_insights', value)}
          />
        </div>
      </Card>

      {/* Privacy Info */}
      <Card variant="dark" className="!p-6 text-center">
        <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-pierre-violet/20 flex items-center justify-center">
          <svg className="w-6 h-6 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
        </div>
        <h3 className="text-lg font-semibold text-white mb-2">Your Privacy is Protected</h3>
        <p className="text-sm text-zinc-400 max-w-md mx-auto">
          When you share insights, your private data is automatically sanitized. GPS coordinates,
          exact pace, recovery scores, and other sensitive information is never shared with friends.
        </p>
      </Card>
    </div>
  );
}
