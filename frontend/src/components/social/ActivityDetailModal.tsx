// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal displaying activity details with Share with Friends functionality
// ABOUTME: Shows activity stats, AI insights, and allows coach-mediated sharing

import { useState } from 'react';
import { Button, Modal, Card } from '../ui';
import ShareInsightModal from './ShareInsightModal';

interface ActivityDetailModalProps {
  /** Activity ID for fetching activity-specific suggestions */
  activityId: string;
  /** Activity title/name to display */
  activityTitle?: string;
  /** Activity type (run, bike, swim, etc.) */
  activityType?: string;
  /** Activity date string */
  activityDate?: string;
  /** The insight content that triggered viewing this activity */
  insightContent?: string;
  /** Called when modal is closed */
  onClose: () => void;
}

// Activity type icons (SVG paths)
const ACTIVITY_ICONS: Record<string, string> = {
  run: 'M13 5.5V7h2v1.5l-2 2V14l2 2h-3l-2.5-2v-4l-2-2V6.5l2-2h3V5.5h.5zm-5 0v1h-2V8l2 2v4l-2 2h3l2.5-2v-4l2-2V6.5l-2-2h-3v1h-.5z',
  bike: 'M15.5 5.5c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zM5 12c-2.8 0-5 2.2-5 5s2.2 5 5 5 5-2.2 5-5-2.2-5-5-5zm0 8.5c-1.9 0-3.5-1.6-3.5-3.5s1.6-3.5 3.5-3.5 3.5 1.6 3.5 3.5-1.6 3.5-3.5 3.5zm5.8-10l2.4-2.4.8.8c1.3 1.3 3 2.1 5.1 2.1V9c-1.5 0-2.7-.6-3.6-1.5l-1.9-1.9c-.5-.4-1-.6-1.6-.6s-1.1.2-1.4.6L7.8 8.4c-.4.4-.6.9-.6 1.4 0 .6.2 1.1.6 1.4L11 14v5h2v-6.2l-2.2-2.3zM19 12c-2.8 0-5 2.2-5 5s2.2 5 5 5 5-2.2 5-5-2.2-5-5-5zm0 8.5c-1.9 0-3.5-1.6-3.5-3.5s1.6-3.5 3.5-3.5 3.5 1.6 3.5 3.5-1.6 3.5-3.5 3.5z',
  swim: 'M22 21c-1.11 0-1.73-.37-2.18-.64-.37-.22-.6-.36-1.15-.36-.56 0-.78.13-1.15.36-.46.27-1.07.64-2.18.64s-1.73-.37-2.18-.64c-.37-.22-.6-.36-1.15-.36-.56 0-.78.13-1.15.36-.46.27-1.08.64-2.19.64-1.11 0-1.73-.37-2.18-.64-.37-.23-.6-.36-1.15-.36s-.78.13-1.15.36c-.46.27-1.08.64-2.19.64v-2c.56 0 .78-.13 1.15-.36.46-.27 1.08-.64 2.19-.64s1.73.37 2.18.64c.37.23.59.36 1.15.36.56 0 .78-.13 1.15-.36.46-.27 1.08-.64 2.19-.64 1.11 0 1.73.37 2.18.64.37.22.6.36 1.15.36s.78-.13 1.15-.36c.45-.27 1.07-.64 2.18-.64s1.73.37 2.18.64c.37.22.6.36 1.15.36v2zm0-4.5c-1.11 0-1.73-.37-2.18-.64-.37-.22-.6-.36-1.15-.36-.56 0-.78.13-1.15.36-.45.27-1.07.64-2.18.64s-1.73-.37-2.18-.64c-.37-.22-.6-.36-1.15-.36-.56 0-.78.13-1.15.36-.45.27-1.07.64-2.18.64s-1.73-.37-2.18-.64c-.37-.22-.6-.36-1.15-.36s-.78.13-1.15.36c-.47.27-1.09.64-2.2.64v-2c.56 0 .78-.13 1.15-.36.45-.27 1.07-.64 2.18-.64s1.73.37 2.18.64c.37.22.6.36 1.15.36.56 0 .78-.13 1.15-.36.45-.27 1.07-.64 2.18-.64s1.73.37 2.18.64c.37.22.6.36 1.15.36s.78-.13 1.15-.36c.45-.27 1.07-.64 2.18-.64s1.73.37 2.18.64c.37.22.6.36 1.15.36v2zM8.67 12c.56 0 .78-.13 1.15-.36.46-.27 1.08-.64 2.19-.64 1.11 0 1.73.37 2.18.64.37.22.6.36 1.15.36s.78-.13 1.15-.36c.12-.07.26-.15.41-.23L10.48 5C10.15 4.37 9.5 4 8.78 4c-.72 0-1.37.37-1.7 1l-2.4 4.16-.06.11C4.22 10.06 4 11.12 4 12v.5h.5c.56 0 .78-.13 1.15-.36.45-.27 1.07-.64 2.18-.64.56 0 .78.13 1.15.36.32.19.72.42 1.24.54L8.67 12z',
  default: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z',
};

export default function ActivityDetailModal({
  activityId,
  activityTitle = 'Activity',
  activityType = 'run',
  activityDate,
  insightContent,
  onClose,
}: ActivityDetailModalProps) {
  const [showShareModal, setShowShareModal] = useState(false);

  const handleShareWithFriends = () => {
    setShowShareModal(true);
  };

  const handleShareSuccess = () => {
    setShowShareModal(false);
    onClose();
  };

  const iconPath = ACTIVITY_ICONS[activityType.toLowerCase()] || ACTIVITY_ICONS.default;

  return (
    <>
      <Modal
        isOpen={true}
        onClose={onClose}
        title="Activity Details"
        size="lg"
      >
        <div className="space-y-6">
          {/* Activity Header */}
          <div className="flex items-start gap-4">
            <div className="w-12 h-12 rounded-xl bg-pierre-violet/20 flex items-center justify-center flex-shrink-0">
              <svg className="w-6 h-6 text-pierre-violet" viewBox="0 0 24 24" fill="currentColor">
                <path d={iconPath} />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <h3 className="text-lg font-semibold text-white truncate">{activityTitle}</h3>
              {activityDate && (
                <p className="text-sm text-zinc-400">{activityDate}</p>
              )}
              <span className="inline-block mt-1 px-2 py-0.5 text-xs font-medium bg-pierre-cyan/20 text-pierre-cyan rounded-full capitalize">
                {activityType}
              </span>
            </div>
          </div>

          {/* AI Insight Card */}
          {insightContent && (
            <Card variant="dark" className="border border-pierre-violet/30">
              <div className="flex items-start gap-3">
                <div className="w-8 h-8 rounded-full bg-pierre-violet/20 flex items-center justify-center flex-shrink-0">
                  <svg className="w-4 h-4 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                </div>
                <div className="flex-1">
                  <p className="text-xs font-medium text-pierre-violet mb-1">AI Insight</p>
                  <p className="text-sm text-zinc-300 leading-relaxed">{insightContent}</p>
                </div>
              </div>
            </Card>
          )}

          {/* Share CTA Section */}
          <Card variant="dark" className="bg-gradient-to-r from-pierre-violet/10 to-pierre-cyan/10 border border-white/10">
            <div className="text-center">
              <h4 className="text-base font-medium text-white mb-2">
                Share this activity with friends
              </h4>
              <p className="text-sm text-zinc-400 mb-4">
                Let Pierre create a coach-generated insight to share with your training partners.
                Your private data stays private - only the insight is shared.
              </p>
              <Button
                variant="primary"
                onClick={handleShareWithFriends}
                className="inline-flex items-center gap-2"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
                Share with Friends
              </Button>
            </div>
          </Card>

          {/* Footer */}
          <div className="flex justify-end pt-2">
            <Button variant="secondary" onClick={onClose}>
              Close
            </Button>
          </div>
        </div>
      </Modal>

      {/* Share Insight Modal */}
      {showShareModal && (
        <ShareInsightModal
          activityId={activityId}
          onClose={() => setShowShareModal(false)}
          onSuccess={handleShareSuccess}
        />
      )}
    </>
  );
}
