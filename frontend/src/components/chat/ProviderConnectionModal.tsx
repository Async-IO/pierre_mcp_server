// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for connecting fitness data providers during chat
// ABOUTME: Displays provider connection cards with skip option

import ProviderConnectionCards from '../ProviderConnectionCards';

interface ProviderConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConnectProvider: (provider: string) => void;
  connectingProvider: string | null;
  onSkip: () => void;
  isSkipPending: boolean;
}

export default function ProviderConnectionModal({
  isOpen,
  onClose,
  onConnectProvider,
  connectingProvider,
  onSkip,
  isSkipPending,
}: ProviderConnectionModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/70 backdrop-blur-sm"
        onClick={onClose}
      />
      {/* Modal Content */}
      <div className="relative bg-pierre-slate rounded-2xl shadow-2xl border border-white/10 max-w-3xl w-full mx-4 max-h-[90vh] overflow-y-auto">
        <div className="p-8">
          {/* Close button */}
          <button
            onClick={onClose}
            className="absolute top-4 right-4 p-2 text-zinc-500 hover:text-white hover:bg-white/10 rounded-lg transition-colors"
            aria-label="Close"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>

          <div className="text-center mb-8">
            <div className="w-14 h-14 bg-pierre-violet/20 rounded-xl flex items-center justify-center mx-auto mb-4 shadow-glow-sm">
              <svg className="w-7 h-7 text-pierre-violet" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
            <h2 className="text-2xl font-semibold text-white mb-3">
              Connect your fitness data
            </h2>
            <p className="text-zinc-400">
              Link a provider for personalized insights, or continue without
            </p>
          </div>

          <ProviderConnectionCards
            onConnectProvider={onConnectProvider}
            connectingProvider={connectingProvider}
            onSkip={onSkip}
            isSkipPending={isSkipPending}
          />
        </div>
      </div>
    </div>
  );
}
