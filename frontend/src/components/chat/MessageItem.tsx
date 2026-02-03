// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Individual message item in the chat message list
// ABOUTME: Memoized for performance when rendering many messages

import { memo } from 'react';
import Markdown from 'react-markdown';
import { Copy, Share2, Users, ThumbsUp, ThumbsDown, RefreshCw, Lightbulb } from 'lucide-react';
import type { Message, MessageMetadata, MessageFeedback } from './types';
import { linkifyUrls, stripContextPrefix } from './utils';

interface MessageItemProps {
  message: Message;
  metadata?: MessageMetadata;
  feedback?: MessageFeedback;
  isError?: boolean;
  hasInsight?: boolean;
  onCopy?: () => void;
  onShare?: () => void;
  onShareToFeed?: () => void;
  onCreateInsight?: () => void;
  onThumbsUp?: () => void;
  onThumbsDown?: () => void;
  onRetry?: () => void;
}

const MessageItem = memo(function MessageItem({
  message,
  metadata,
  feedback,
  isError = false,
  hasInsight = false,
  onCopy,
  onShare,
  onShareToFeed,
  onCreateInsight,
  onThumbsUp,
  onThumbsDown,
  onRetry,
}: MessageItemProps) {
  const isUser = message.role === 'user';
  const content = stripContextPrefix(message.content);

  return (
    <div className="flex gap-3">
      {/* Avatar */}
      <div className="flex-shrink-0">
        {isUser ? (
          <div className="w-8 h-8 rounded-full bg-white/10 flex items-center justify-center">
            <svg className="w-4 h-4 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
          </div>
        ) : (
          <img src="/pierre-icon.svg" alt="Pierre" className="w-8 h-8 rounded-xl" />
        )}
      </div>
      {/* Message Content */}
      <div className="flex-1 min-w-0 pt-1">
        <div className="font-medium text-white text-sm mb-1">
          {isUser ? 'You' : 'Pierre'}
        </div>
        <div className={`text-zinc-300 text-sm leading-relaxed prose prose-sm prose-invert max-w-none prose-a:text-pierre-violet prose-a:underline hover:prose-a:text-pierre-violet/80 ${isError ? 'text-red-400' : ''}`}>
          <Markdown
            components={{
              a: ({ href, children }) => (
                <a href={href} target="_blank" rel="noopener noreferrer" className="break-all">
                  {children}
                </a>
              ),
            }}
          >
            {linkifyUrls(content)}
          </Markdown>
        </div>
        {/* Action icons and metadata for assistant messages - matches mobile design */}
        {!isUser && (
          <div className="mt-2 flex items-center gap-4">
            {isError ? (
              /* For error messages, show only Retry button with label */
              <button
                onClick={onRetry}
                className="flex items-center gap-1 px-2 py-1 text-xs text-white font-medium bg-white/10 rounded hover:bg-white/15 transition-colors"
              >
                <RefreshCw className="w-3.5 h-3.5" />
                <span>Retry</span>
              </button>
            ) : (
              /* Normal assistant message actions */
              <>
                {/* Copy */}
                {onCopy && (
                  <button
                    onClick={onCopy}
                    className="p-0.5 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title="Copy message"
                  >
                    <Copy className="w-3.5 h-3.5" />
                  </button>
                )}
                {/* Share - always visible */}
                {onShare && (
                  <button
                    onClick={onShare}
                    className="p-0.5 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title="Share"
                  >
                    <Share2 className="w-3.5 h-3.5" />
                  </button>
                )}
                {/* Create Insight - only for non-insight messages */}
                {onCreateInsight && !hasInsight && (
                  <button
                    onClick={onCreateInsight}
                    className="p-0.5 text-zinc-500 hover:text-pierre-cyan transition-colors"
                    title="Create shareable insight"
                  >
                    <Lightbulb className="w-3.5 h-3.5" />
                  </button>
                )}
                {/* Share to Feed - only for insight messages */}
                {onShareToFeed && hasInsight && (
                  <button
                    onClick={onShareToFeed}
                    className="p-0.5 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title="Share to Feed"
                  >
                    <Users className="w-3.5 h-3.5" />
                  </button>
                )}
                {/* Thumbs Up */}
                {onThumbsUp && (
                  <button
                    onClick={onThumbsUp}
                    className={`p-0.5 transition-colors ${
                      feedback === 'up' ? 'text-pierre-violet' : 'text-zinc-500 hover:text-zinc-300'
                    }`}
                    title="Good response"
                  >
                    <ThumbsUp className={`w-3.5 h-3.5 ${feedback === 'up' ? 'fill-current' : ''}`} />
                  </button>
                )}
                {/* Thumbs Down */}
                {onThumbsDown && (
                  <button
                    onClick={onThumbsDown}
                    className={`p-0.5 transition-colors ${
                      feedback === 'down' ? 'text-red-500' : 'text-zinc-500 hover:text-zinc-300'
                    }`}
                    title="Poor response"
                  >
                    <ThumbsDown className={`w-3.5 h-3.5 ${feedback === 'down' ? 'fill-current' : ''}`} />
                  </button>
                )}
                {/* Retry */}
                {onRetry && (
                  <button
                    onClick={onRetry}
                    className="p-0.5 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title="Regenerate response"
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                  </button>
                )}
                {/* Model and response time - to the right of icons */}
                {metadata && (
                  <span className="text-xs text-zinc-500 ml-2">
                    {metadata.model}{metadata.executionTimeMs ? ` · ${(metadata.executionTimeMs / 1000).toFixed(1)}s` : ''}
                  </span>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
});

export default MessageItem;
