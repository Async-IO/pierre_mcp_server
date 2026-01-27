// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Chat message input component with textarea and send button
// ABOUTME: Handles keyboard shortcuts and ideas popover

import { useRef, useEffect } from 'react';
import { clsx } from 'clsx';
import PromptSuggestions from '../PromptSuggestions';

interface MessageInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  isStreaming: boolean;
  showIdeas: boolean;
  onToggleIdeas: () => void;
  onSelectPrompt: (prompt: string, systemPrompt?: string) => void;
}

export default function MessageInput({
  value,
  onChange,
  onSend,
  isStreaming,
  showIdeas,
  onToggleIdeas,
  onSelectPrompt,
}: MessageInputProps) {
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter to send (without shift)
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  };

  return (
    <div className="border-t border-white/10 p-4 bg-pierre-slate">
      <div className="max-w-3xl mx-auto">
        {/* Ideas popover */}
        {showIdeas && (
          <div className="mb-4 p-4 bg-white/5 rounded-xl border border-white/10 relative">
            <button
              onClick={onToggleIdeas}
              className="absolute top-2 right-2 text-zinc-500 hover:text-white transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
            <p className="text-xs text-zinc-400 mb-3">Click a suggestion to fill the input:</p>
            <PromptSuggestions onSelectPrompt={onSelectPrompt} />
          </div>
        )}
        <div className="relative">
          <textarea
            ref={inputRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Message Pierre..."
            className="w-full resize-none rounded-xl border border-white/10 bg-[#151520] text-white placeholder-zinc-500 pl-4 pr-14 py-3 focus:outline-none focus:ring-2 focus:ring-pierre-violet/30 focus:border-pierre-violet text-sm transition-colors overflow-hidden"
            rows={1}
            disabled={isStreaming}
          />
          <button
            onClick={onSend}
            disabled={!value.trim() || isStreaming}
            aria-label="Send message"
            className={clsx(
              'absolute right-3 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center rounded-lg transition-colors',
              value.trim() && !isStreaming
                ? 'bg-pierre-violet text-white hover:bg-pierre-violet/90 shadow-glow-sm'
                : 'text-zinc-600 cursor-not-allowed'
            )}
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
            </svg>
          </button>
        </div>
        <div className="flex items-center justify-center gap-2 mt-2">
          <p className="text-xs text-zinc-500">
            Press Enter to send, Shift+Enter for new line
          </p>
          <span className="text-zinc-600">|</span>
          <button
            onClick={onToggleIdeas}
            className="text-xs text-pierre-violet hover:text-pierre-violet/80 flex items-center gap-1 transition-colors"
          >
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
            </svg>
            Need ideas?
          </button>
        </div>
      </div>
    </div>
  );
}
