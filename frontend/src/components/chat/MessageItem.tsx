// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Individual message item in the chat message list
// ABOUTME: Memoized for performance when rendering many messages

import { memo } from 'react';
import Markdown from 'react-markdown';
import { Share2 } from 'lucide-react';
import type { Message, MessageMetadata } from './types';
import { linkifyUrls, stripContextPrefix } from './utils';

interface MessageItemProps {
  message: Message;
  metadata?: MessageMetadata;
  onShare?: () => void;
}

const MessageItem = memo(function MessageItem({
  message,
  metadata,
  onShare,
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
            {linkifyUrls(content)}
          </Markdown>
        </div>
        {/* Model and execution time metadata for assistant messages */}
        {!isUser && metadata && (
          <div className="mt-2 text-xs text-zinc-500 flex items-center gap-2">
            <span className="inline-flex items-center gap-1">
              <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
              </svg>
              {metadata.model}
            </span>
            <span className="inline-flex items-center gap-1">
              <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              {(metadata.executionTimeMs / 1000).toFixed(1)}s
            </span>
          </div>
        )}
        {/* Share button for assistant messages */}
        {!isUser && onShare && (
          <div className="mt-2 flex items-center gap-2">
            <button
              onClick={onShare}
              className="flex items-center gap-1 text-xs text-zinc-400 hover:text-pierre-violet transition-colors"
              title="Share to social feed"
            >
              <Share2 className="w-3.5 h-3.5" />
              <span>Share</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
});

export default MessageItem;
