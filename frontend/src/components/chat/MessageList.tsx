// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Message list component displaying chat messages with streaming support
// ABOUTME: Handles message display, streaming content, loading states, and errors

import { useRef, useEffect } from 'react';
import Markdown from 'react-markdown';
import MessageItem from './MessageItem';
import type { Message, MessageMetadata, OAuthNotification } from './types';
import { linkifyUrls } from './utils';

interface MessageListProps {
  messages: Message[];
  messageMetadata: Map<string, MessageMetadata>;
  isLoading: boolean;
  isStreaming: boolean;
  streamingContent: string;
  errorMessage: string | null;
  errorCountdown: number | null;
  oauthNotification: OAuthNotification | null;
  onDismissError: () => void;
  onDismissOAuthNotification: () => void;
  onShareMessage: (content: string) => void;
}

export default function MessageList({
  messages,
  messageMetadata,
  isLoading,
  isStreaming,
  streamingContent,
  errorMessage,
  errorCountdown,
  oauthNotification,
  onDismissError,
  onDismissOAuthNotification,
  onShareMessage,
}: MessageListProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingContent]);

  if (isLoading) {
    return (
      <div className="text-center text-zinc-400 py-8 text-sm">Loading messages...</div>
    );
  }

  return (
    <div className="space-y-6">
      {messages.map((msg) => (
        <MessageItem
          key={msg.id}
          message={msg}
          metadata={messageMetadata.get(msg.id)}
          onShare={msg.role === 'assistant' ? () => onShareMessage(msg.content) : undefined}
        />
      ))}

      {/* OAuth connection notification */}
      {oauthNotification && (
        <div className="flex gap-3 animate-fadeIn">
          <div className="flex-shrink-0">
            <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
          </div>
          <div className="flex-1 min-w-0 pt-1">
            <div className="font-medium text-white text-sm mb-1 flex items-center gap-2">
              Pierre
              <button
                onClick={onDismissOAuthNotification}
                className="text-zinc-500 hover:text-white transition-colors"
              >
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
            <div className="text-zinc-300 text-sm leading-relaxed">
              {oauthNotification.provider} connected successfully. You can now access your {oauthNotification.provider} data.
            </div>
          </div>
        </div>
      )}

      {/* Streaming response */}
      {isStreaming && streamingContent && (
        <div className="flex gap-3">
          <div className="flex-shrink-0">
            <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
          </div>
          <div className="flex-1 min-w-0 pt-1">
            <div className="font-medium text-white text-sm mb-1 flex items-center gap-2">
              Pierre
              <span className="w-1.5 h-1.5 bg-pierre-violet rounded-full animate-pulse" />
            </div>
            <div className="text-zinc-300 text-sm leading-relaxed prose prose-sm prose-invert max-w-none prose-a:text-pierre-violet prose-a:underline hover:prose-a:text-pierre-violet/80">
              <Markdown
                components={{
                  a: ({ href, children }) => (
                    <a href={href} target="_blank" rel="noopener noreferrer" className="break-all">
                      {children}
                    </a>
                  ),
                }}
              >
                {linkifyUrls(streamingContent)}
              </Markdown>
            </div>
          </div>
        </div>
      )}

      {/* Thinking/Loading indicator */}
      {isStreaming && !streamingContent && (
        <div className="flex gap-3">
          <div className="flex-shrink-0">
            <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
          </div>
          <div className="flex-1 pt-1">
            <div className="font-medium text-white text-sm mb-2 flex items-center gap-2">
              Pierre
            </div>
            <div className="flex items-center gap-2 text-zinc-400 text-sm">
              <div className="pierre-spinner w-4 h-4"></div>
              <span>Thinking...</span>
            </div>
          </div>
        </div>
      )}

      {/* Error message display */}
      {errorMessage && !isStreaming && (
        <div className="flex gap-3">
          <div className="flex-shrink-0">
            <div className="w-8 h-8 rounded-full bg-red-500/20 flex items-center justify-center">
              <svg className="w-4 h-4 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
          </div>
          <div className="flex-1 pt-1">
            <div className="bg-red-500/10 border border-red-500/30 rounded-lg px-4 py-3">
              <p className="text-red-400 text-sm">
                {errorCountdown !== null
                  ? errorMessage.replace(/in \d+ seconds/, `in ${errorCountdown} seconds`)
                  : errorMessage}
              </p>
              <button
                onClick={onDismissError}
                className="text-red-400 hover:text-red-300 text-xs mt-2 underline transition-colors"
              >
                Dismiss
              </button>
            </div>
          </div>
        </div>
      )}

      <div ref={messagesEndRef} />
    </div>
  );
}
