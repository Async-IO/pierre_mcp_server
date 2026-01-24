// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Social feed tab displaying shared coach insights from friends
// ABOUTME: Includes reactions, adapt-to-my-training feature, and infinite scroll

import { useState, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { apiService } from '../../services/api';
import { Card, Button } from '../ui';
import ShareInsightModal from './ShareInsightModal';
import AdaptInsightModal from './AdaptInsightModal';

interface SharedInsight {
  id: string;
  user_id: string;
  visibility: string;
  insight_type: string;
  sport_type: string | null;
  content: string;
  title: string | null;
  training_phase: string | null;
  reaction_count: number;
  adapt_count: number;
  created_at: string;
  updated_at: string;
  expires_at: string | null;
}

interface FeedAuthor {
  user_id: string;
  display_name: string | null;
  email: string;
}

interface ReactionCounts {
  like: number;
  celebrate: number;
  inspire: number;
  support: number;
  total: number;
}

interface FeedItem {
  insight: SharedInsight;
  author: FeedAuthor;
  reactions: ReactionCounts;
  user_reaction: string | null;
  user_has_adapted: boolean;
}

type ReactionType = 'like' | 'celebrate' | 'inspire' | 'support';

const REACTION_CONFIG: Record<ReactionType, { icon: string; color: string; label: string }> = {
  like: { icon: '👍', color: 'text-blue-400', label: 'Like' },
  celebrate: { icon: '🎉', color: 'text-yellow-400', label: 'Celebrate' },
  inspire: { icon: '💪', color: 'text-purple-400', label: 'Inspire' },
  support: { icon: '🤗', color: 'text-red-400', label: 'Support' },
};

const INSIGHT_TYPE_CONFIG: Record<string, { icon: string; color: string; label: string }> = {
  achievement: { icon: '🏆', color: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30', label: 'Achievement' },
  milestone: { icon: '🚩', color: 'bg-amber-500/20 text-amber-400 border-amber-500/30', label: 'Milestone' },
  training_tip: { icon: '⚡', color: 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30', label: 'Training Tip' },
  recovery: { icon: '🌙', color: 'bg-violet-500/20 text-violet-400 border-violet-500/30', label: 'Recovery' },
  motivation: { icon: '☀️', color: 'bg-orange-500/20 text-orange-400 border-orange-500/30', label: 'Motivation' },
};

export default function SocialFeedTab() {
  const [feedItems, setFeedItems] = useState<FeedItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [reactionLoading, setReactionLoading] = useState<string | null>(null);
  const [showShareModal, setShowShareModal] = useState(false);
  const [showAdaptModal, setShowAdaptModal] = useState(false);
  const [selectedInsightId, setSelectedInsightId] = useState<string | null>(null);

  const loadFeed = useCallback(async (cursor?: string) => {
    try {
      if (cursor) {
        setIsLoadingMore(true);
      } else {
        setIsLoading(true);
      }

      const response = await apiService.getSocialFeed(cursor, 20);

      if (cursor) {
        setFeedItems(prev => [...prev, ...response.items]);
      } else {
        setFeedItems(response.items);
      }

      setNextCursor(response.next_cursor);
      setHasMore(response.has_more);
    } catch (error) {
      console.error('Failed to load feed:', error);
    } finally {
      setIsLoading(false);
      setIsLoadingMore(false);
    }
  }, []);

  useEffect(() => {
    loadFeed();
  }, [loadFeed]);

  const handleReaction = async (insightId: string, reactionType: ReactionType) => {
    const item = feedItems.find(i => i.insight.id === insightId);
    if (!item) return;

    try {
      setReactionLoading(insightId);

      if (item.user_reaction === reactionType) {
        // Remove reaction
        await apiService.removeReaction(insightId);
        setFeedItems(prev =>
          prev.map(i => {
            if (i.insight.id === insightId) {
              return {
                ...i,
                user_reaction: null,
                reactions: {
                  ...i.reactions,
                  [reactionType]: Math.max(0, i.reactions[reactionType] - 1),
                  total: Math.max(0, i.reactions.total - 1),
                },
              };
            }
            return i;
          })
        );
      } else {
        // Add or change reaction
        const response = await apiService.addReaction(insightId, reactionType);
        setFeedItems(prev =>
          prev.map(i => {
            if (i.insight.id === insightId) {
              return {
                ...i,
                user_reaction: reactionType,
                reactions: response.updated_counts,
              };
            }
            return i;
          })
        );
      }
    } catch (error) {
      console.error('Failed to update reaction:', error);
    } finally {
      setReactionLoading(null);
    }
  };

  const handleAdapt = (insightId: string) => {
    setSelectedInsightId(insightId);
    setShowAdaptModal(true);
  };

  const handleAdaptSuccess = () => {
    if (selectedInsightId) {
      setFeedItems(prev =>
        prev.map(i => {
          if (i.insight.id === selectedInsightId) {
            return { ...i, user_has_adapted: true };
          }
          return i;
        })
      );
    }
    setShowAdaptModal(false);
    setSelectedInsightId(null);
  };

  const handleShareSuccess = () => {
    setShowShareModal(false);
    loadFeed(); // Refresh feed to show new post
  };

  const getInitials = (name: string | null, email: string): string => {
    if (name) {
      const parts = name.split(' ');
      if (parts.length >= 2) {
        return (parts[0][0] + parts[1][0]).toUpperCase();
      }
      return name.substring(0, 2).toUpperCase();
    }
    return email.substring(0, 2).toUpperCase();
  };

  const getAvatarColor = (str: string): string => {
    const hash = str.split('').reduce((acc, char) => {
      return char.charCodeAt(0) + ((acc << 5) - acc);
    }, 0);
    const hue = Math.abs(hash % 360);
    return `hsl(${hue}, 70%, 50%)`;
  };

  const formatRelativeTime = (dateStr: string): string => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-white">Social Feed</h2>
          <p className="text-sm text-zinc-400 mt-1">
            Coach insights from your friends
          </p>
        </div>
        <Button variant="primary" onClick={() => setShowShareModal(true)}>
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Share Insight
          </span>
        </Button>
      </div>

      {/* Feed Content */}
      {isLoading ? (
        <div className="flex justify-center py-8">
          <div className="pierre-spinner"></div>
        </div>
      ) : feedItems.length === 0 ? (
        <Card variant="dark" className="!p-8 text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-pierre-violet/20 flex items-center justify-center">
            <svg className="w-8 h-8 text-pierre-violet-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
            </svg>
          </div>
          <h3 className="text-lg font-semibold text-white mb-2">Your feed is empty</h3>
          <p className="text-zinc-400 mb-4">
            Add friends or share your first insight to get started
          </p>
          <Button variant="primary" onClick={() => setShowShareModal(true)}>
            Share Your First Insight
          </Button>
        </Card>
      ) : (
        <div className="space-y-4">
          {feedItems.map((item) => {
            const typeConfig = INSIGHT_TYPE_CONFIG[item.insight.insight_type] || INSIGHT_TYPE_CONFIG.motivation;

            return (
              <Card key={item.insight.id} variant="dark" className="!p-5">
                {/* Header */}
                <div className="flex items-start justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div
                      className="w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold text-white"
                      style={{ backgroundColor: getAvatarColor(item.author.email) }}
                    >
                      {getInitials(item.author.display_name, item.author.email)}
                    </div>
                    <div>
                      <p className="font-medium text-white">
                        {item.author.display_name || item.author.email.split('@')[0]}
                      </p>
                      <p className="text-sm text-zinc-500">
                        {formatRelativeTime(item.insight.created_at)}
                      </p>
                    </div>
                  </div>
                  <span className={clsx(
                    'px-2 py-1 text-xs font-medium rounded-full border flex items-center gap-1',
                    typeConfig.color
                  )}>
                    <span>{typeConfig.icon}</span>
                    <span>{typeConfig.label}</span>
                  </span>
                </div>

                {/* Content */}
                {item.insight.title && (
                  <h3 className="text-lg font-semibold text-white mb-2">{item.insight.title}</h3>
                )}
                <p className="text-zinc-300 mb-4 whitespace-pre-wrap">{item.insight.content}</p>

                {/* Context badges */}
                {(item.insight.sport_type || item.insight.training_phase) && (
                  <div className="flex gap-2 mb-4">
                    {item.insight.sport_type && (
                      <span className="px-2 py-1 text-xs bg-white/10 text-zinc-400 rounded-full">
                        {item.insight.sport_type}
                      </span>
                    )}
                    {item.insight.training_phase && (
                      <span className="px-2 py-1 text-xs bg-white/10 text-zinc-400 rounded-full capitalize">
                        {item.insight.training_phase} phase
                      </span>
                    )}
                  </div>
                )}

                {/* Reactions */}
                <div className="flex items-center justify-between pt-4 border-t border-white/10">
                  <div className="flex gap-2">
                    {(Object.keys(REACTION_CONFIG) as ReactionType[]).map((type) => {
                      const config = REACTION_CONFIG[type];
                      const isActive = item.user_reaction === type;
                      const count = item.reactions[type];

                      return (
                        <button
                          key={type}
                          onClick={() => handleReaction(item.insight.id, type)}
                          disabled={reactionLoading === item.insight.id}
                          className={clsx(
                            'flex items-center gap-1 px-3 py-1.5 rounded-lg text-sm transition-colors',
                            isActive
                              ? 'bg-white/20 text-white'
                              : 'bg-white/5 text-zinc-400 hover:bg-white/10 hover:text-white'
                          )}
                          title={config.label}
                        >
                          <span>{config.icon}</span>
                          {count > 0 && <span className="text-xs">{count}</span>}
                        </button>
                      );
                    })}
                  </div>

                  {/* Adapt button */}
                  <Button
                    variant={item.user_has_adapted ? 'secondary' : 'primary'}
                    size="sm"
                    onClick={() => handleAdapt(item.insight.id)}
                    disabled={item.user_has_adapted}
                  >
                    {item.user_has_adapted ? (
                      <span className="flex items-center gap-2">
                        <svg className="w-4 h-4 text-pierre-activity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                        Adapted
                      </span>
                    ) : (
                      <span className="flex items-center gap-2">
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        Adapt to My Training
                      </span>
                    )}
                  </Button>
                </div>
              </Card>
            );
          })}

          {/* Load More */}
          {hasMore && (
            <div className="flex justify-center pt-4">
              <Button
                variant="secondary"
                onClick={() => nextCursor && loadFeed(nextCursor)}
                loading={isLoadingMore}
              >
                Load More
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Modals */}
      {showShareModal && (
        <ShareInsightModal
          onClose={() => setShowShareModal(false)}
          onSuccess={handleShareSuccess}
        />
      )}

      {showAdaptModal && selectedInsightId && (
        <AdaptInsightModal
          insightId={selectedInsightId}
          onClose={() => {
            setShowAdaptModal(false);
            setSelectedInsightId(null);
          }}
          onSuccess={handleAdaptSuccess}
        />
      )}
    </div>
  );
}
