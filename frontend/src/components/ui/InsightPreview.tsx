// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Reusable component for displaying insight/message content with markdown rendering
// ABOUTME: Provides consistent styling for content preview across modals and cards

import Markdown from 'react-markdown';

interface InsightPreviewProps {
  /** The markdown content to render */
  content: string;
  /** Maximum height before scrolling (default: 384px / max-h-96) */
  maxHeight?: string;
  /** Optional label above the content */
  label?: string;
  /** Optional CSS class for the container */
  className?: string;
}

export function InsightPreview({
  content,
  maxHeight = 'max-h-96',
  label,
  className = '',
}: InsightPreviewProps) {
  return (
    <div className={className}>
      {label && (
        <label className="block text-sm font-medium text-zinc-300 mb-2">
          {label}
        </label>
      )}
      <div
        className={`bg-white/5 rounded-lg p-4 text-zinc-100 text-sm ${maxHeight} overflow-y-auto prose prose-sm prose-invert max-w-none prose-headings:text-zinc-100 prose-headings:font-semibold prose-headings:mt-4 prose-headings:mb-2 prose-strong:text-zinc-100 prose-ul:my-2 prose-li:my-0.5 prose-p:my-2`}
      >
        <Markdown
          components={{
            a: ({ href, children }) => (
              <a
                href={href}
                target="_blank"
                rel="noopener noreferrer"
                className="text-pierre-violet underline hover:text-pierre-violet/80 break-all"
              >
                {children}
              </a>
            ),
            // Ensure headings have proper styling
            h1: ({ children }) => (
              <h1 className="text-lg font-bold text-zinc-100 mt-4 mb-2">{children}</h1>
            ),
            h2: ({ children }) => (
              <h2 className="text-base font-bold text-zinc-100 mt-4 mb-2">{children}</h2>
            ),
            h3: ({ children }) => (
              <h3 className="text-sm font-bold text-zinc-100 mt-3 mb-1">{children}</h3>
            ),
            // Lists
            ul: ({ children }) => (
              <ul className="list-disc list-inside my-2 space-y-1">{children}</ul>
            ),
            ol: ({ children }) => (
              <ol className="list-decimal list-inside my-2 space-y-1">{children}</ol>
            ),
            // Code blocks
            code: ({ children, className }) => {
              const isInline = !className;
              return isInline ? (
                <code className="bg-white/10 px-1.5 py-0.5 rounded text-pierre-cyan text-xs">
                  {children}
                </code>
              ) : (
                <code className="block bg-white/10 p-3 rounded-lg text-xs overflow-x-auto">
                  {children}
                </code>
              );
            },
          }}
        >
          {content}
        </Markdown>
      </div>
    </div>
  );
}
