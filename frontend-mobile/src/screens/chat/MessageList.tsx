// ABOUTME: Message list component with FlatList rendering and empty states
// ABOUTME: Handles message display, thinking indicator, and coach grid for new chats

import React from 'react';
import {
  View,
  Text,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  ScrollView,
  Platform,
  Image,
} from 'react-native';
import Markdown from 'react-native-markdown-display';
import { Ionicons } from '@expo/vector-icons';
import * as Clipboard from 'expo-clipboard';
import { Alert, Share } from 'react-native';
import { colors, spacing, fontSize, borderRadius, aiGlow } from '../../constants/theme';
import type { Message, Coach } from '../../types';

// Coach category badge background colors
const COACH_CATEGORY_BADGE_BG: Record<string, string> = {
  training: 'rgba(16, 185, 129, 0.15)',
  nutrition: 'rgba(245, 158, 11, 0.15)',
  recovery: 'rgba(99, 102, 241, 0.15)',
  recipes: 'rgba(249, 115, 22, 0.15)',
  mobility: 'rgba(236, 72, 153, 0.15)',
  custom: 'rgba(124, 58, 237, 0.15)',
};

// Coach category emoji icons
const COACH_CATEGORY_ICONS: Record<string, string> = {
  training: '🏃',
  nutrition: '🥗',
  recovery: '😴',
  recipes: '👨‍🍳',
  mobility: '🧘',
  custom: '⚙️',
};

// Markdown styles for assistant messages
const markdownStyles = {
  body: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    lineHeight: fontSize.md * 1.5,
  },
  heading1: {
    color: colors.text.primary,
    fontSize: fontSize.xl,
    fontWeight: '700' as const,
    marginTop: spacing.md,
    marginBottom: spacing.sm,
  },
  heading2: {
    color: colors.text.primary,
    fontSize: fontSize.lg,
    fontWeight: '600' as const,
    marginTop: spacing.sm,
    marginBottom: spacing.xs,
  },
  heading3: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600' as const,
    marginTop: spacing.xs,
    marginBottom: spacing.xs,
  },
  strong: {
    color: colors.text.primary,
    fontWeight: '700' as const,
  },
  em: {
    color: colors.text.secondary,
    fontStyle: 'italic' as const,
  },
  bullet_list: {
    marginLeft: spacing.sm,
  },
  ordered_list: {
    marginLeft: spacing.sm,
  },
  list_item: {
    marginBottom: spacing.xs,
  },
  code_inline: {
    backgroundColor: colors.background.tertiary,
    color: colors.primary[400],
    paddingHorizontal: 4,
    borderRadius: 4,
    fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
    fontSize: fontSize.sm,
  },
  fence: {
    backgroundColor: colors.background.tertiary,
    borderRadius: borderRadius.sm,
    padding: spacing.sm,
    marginVertical: spacing.xs,
  },
  code_block: {
    fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
    fontSize: fontSize.sm,
    color: colors.text.primary,
  },
  link: {
    color: colors.primary[400],
    textDecorationLine: 'underline' as const,
  },
  hr: {
    backgroundColor: colors.border.default,
    height: 1,
    marginVertical: spacing.sm,
  },
};

// Helper to detect OAuth authorization URLs
const isOAuthUrl = (url: string): { isOAuth: boolean; provider: string | null } => {
  try {
    const parsedUrl = new URL(url);
    const hostname = parsedUrl.hostname.toLowerCase();

    if (hostname === 'www.strava.com' || hostname === 'strava.com') {
      if (parsedUrl.pathname.includes('/oauth/authorize')) {
        return { isOAuth: true, provider: 'Strava' };
      }
    }
    if (hostname === 'www.fitbit.com' || hostname === 'fitbit.com') {
      if (parsedUrl.pathname.includes('/oauth2/authorize')) {
        return { isOAuth: true, provider: 'Fitbit' };
      }
    }
    if (hostname.endsWith('.garmin.com') || hostname === 'garmin.com') {
      if (url.includes('oauth')) {
        return { isOAuth: true, provider: 'Garmin' };
      }
    }
    return { isOAuth: false, provider: null };
  } catch {
    return { isOAuth: false, provider: null };
  }
};

interface MessageListProps {
  messages: Message[];
  coaches: Coach[];
  isLoading: boolean;
  isSending: boolean;
  isCoachConversation: boolean;
  messageFeedback: Record<string, 'up' | 'down' | null>;
  insightMessages: Set<string>;
  flatListRef: React.RefObject<FlatList | null>;
  onScrollToBottom: () => void;
  onCoachSelect: (coach: Coach) => void;
  onCreateInsight: (content: string) => void;
  onShareToFeed: (content: string) => void;
  onThumbsUp: (messageId: string) => void;
  onThumbsDown: (messageId: string) => void;
  onRetryMessage: (messageId: string) => void;
  onOpenUrl: (url: string) => void;
}

export function MessageList({
  messages,
  coaches,
  isLoading,
  isSending,
  isCoachConversation,
  messageFeedback,
  insightMessages,
  flatListRef,
  onScrollToBottom,
  onCoachSelect,
  onCreateInsight,
  onShareToFeed,
  onThumbsUp,
  onThumbsDown,
  onRetryMessage,
  onOpenUrl,
}: MessageListProps) {
  const handleCopyMessage = async (content: string) => {
    try {
      await Clipboard.setStringAsync(content);
      Alert.alert('Copied', 'Message copied to clipboard');
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  const handleShareMessage = async (content: string) => {
    try {
      await Share.share({ message: content });
    } catch (error) {
      console.error('Failed to share:', error);
    }
  };

  const renderMessageContent = (content: string, isUser: boolean) => {
    if (isUser) {
      return (
        <Text className="text-base text-text-primary leading-6">
          {content}
        </Text>
      );
    }

    const urlRegex = /https?:\/\/[^\s<>"\]]+/gi;
    const oauthUrls = content.match(urlRegex)?.filter(url => {
      const { isOAuth } = isOAuthUrl(url);
      return isOAuth;
    }) || [];

    if (oauthUrls.length > 0) {
      let cleanContent = content;
      oauthUrls.forEach(url => {
        const escapedUrl = url.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        cleanContent = cleanContent.replace(new RegExp(`!\\[([^\\]]*)\\]\\(${escapedUrl}\\)`, 'g'), '');
        cleanContent = cleanContent.replace(new RegExp(`\\[([^\\]]*)\\]\\(${escapedUrl}\\)`, 'g'), '');
        cleanContent = cleanContent.replace(url, '');
      });

      return (
        <View className="flex-row flex-wrap items-center">
          {oauthUrls.map((url, index) => {
            const { provider } = isOAuthUrl(url);
            return (
              <TouchableOpacity
                key={`oauth-${index}`}
                className="px-4 py-2 rounded-lg my-1 self-start"
                style={{ backgroundColor: colors.providers.strava }}
                onPress={() => onOpenUrl(url)}
              >
                <Text className="text-base font-semibold text-text-primary">
                  Connect to {provider}
                </Text>
              </TouchableOpacity>
            );
          })}
          {cleanContent.trim() && (
            <Markdown style={markdownStyles} onLinkPress={(url) => { onOpenUrl(url); return false; }}>
              {cleanContent.trim()}
            </Markdown>
          )}
        </View>
      );
    }

    return (
      <Markdown style={markdownStyles} onLinkPress={(url) => { onOpenUrl(url); return false; }}>
        {content}
      </Markdown>
    );
  };

  const renderMessage = ({ item }: { item: Message }) => {
    if (!item?.id) return null;

    const isUser = item.role === 'user';
    const isError = item.isError === true;
    const timestamp = item.created_at ? new Date(item.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';

    return (
      <View className={`mb-4 ${isUser ? 'items-end' : ''}`}>
        <Text className="text-xs text-zinc-500 mb-1 px-1">{timestamp}</Text>
        <View
          className={`flex-row max-w-[85%] rounded-2xl p-4 ${
            isUser ? 'rounded-br-[4px]' : 'rounded-bl-[4px]'
          } ${isError ? 'border border-error' : ''}`}
          style={[
            isUser ? { backgroundColor: colors.pierre.violet } : undefined,
            !isUser && !isError ? {
              backgroundColor: 'rgba(30, 30, 46, 0.9)',
              borderWidth: 1,
              borderColor: 'rgba(139, 92, 246, 0.2)',
              ...aiGlow.ambient,
            } : undefined,
            isError ? { backgroundColor: 'rgba(239, 68, 68, 0.15)' } : undefined,
          ]}
        >
          {!isUser && (
            <View className="w-8 h-8 rounded-full mr-2 overflow-hidden">
              <Image
                source={require('../../../assets/pierre-logo.png')}
                className="w-8 h-8"
                resizeMode="cover"
              />
            </View>
          )}
          <View className="flex-1">
            {renderMessageContent(item.content, isUser)}
          </View>
        </View>
        {!isUser && (
          <View className="flex-row mt-1 gap-4">
            {isError ? (
              <TouchableOpacity
                className="flex-row items-center bg-background-tertiary px-2 py-1 rounded gap-1"
                onPress={() => onRetryMessage(item.id)}
              >
                <Ionicons name="refresh-outline" size={14} color={colors.text.primary} />
                <Text className="text-xs text-text-primary font-medium">Retry</Text>
              </TouchableOpacity>
            ) : (
              <>
                <TouchableOpacity className="p-0.5" onPress={() => handleCopyMessage(item.content)}>
                  <Ionicons name="copy-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                <TouchableOpacity className="p-0.5" onPress={() => handleShareMessage(item.content)}>
                  <Ionicons name="arrow-redo-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {!insightMessages.has(item.id) && (
                  <TouchableOpacity className="p-0.5" onPress={() => onCreateInsight(item.content)}>
                    <Ionicons name="bulb-outline" size={14} color={colors.text.tertiary} />
                  </TouchableOpacity>
                )}
                {insightMessages.has(item.id) && (
                  <TouchableOpacity className="p-0.5" onPress={() => onShareToFeed(item.content)}>
                    <Ionicons name="people-outline" size={14} color={colors.text.tertiary} />
                  </TouchableOpacity>
                )}
                <TouchableOpacity className="p-0.5" onPress={() => onThumbsUp(item.id)}>
                  <Ionicons
                    name={messageFeedback[item.id] === 'up' ? 'thumbs-up' : 'thumbs-up-outline'}
                    size={14}
                    color={messageFeedback[item.id] === 'up' ? colors.pierre.violet : colors.text.tertiary}
                  />
                </TouchableOpacity>
                <TouchableOpacity className="p-0.5" onPress={() => onThumbsDown(item.id)}>
                  <Ionicons
                    name={messageFeedback[item.id] === 'down' ? 'thumbs-down' : 'thumbs-down-outline'}
                    size={14}
                    color={messageFeedback[item.id] === 'down' ? colors.error : colors.text.tertiary}
                  />
                </TouchableOpacity>
                <TouchableOpacity className="p-0.5" onPress={() => onRetryMessage(item.id)}>
                  <Ionicons name="refresh-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {item.model && (
                  <Text className="text-xs text-text-tertiary ml-2">
                    {item.model}{item.execution_time_ms ? ` · ${(item.execution_time_ms / 1000).toFixed(1)}s` : ''}
                  </Text>
                )}
              </>
            )}
          </View>
        )}
      </View>
    );
  };

  const renderThinkingIndicator = () => (
    <View className="mb-4" testID="thinking-indicator">
      <View
        className="flex-row max-w-[85%] rounded-2xl rounded-bl-[4px] p-4"
        style={{
          backgroundColor: 'rgba(30, 30, 46, 0.9)',
          borderWidth: 1,
          borderColor: 'rgba(139, 92, 246, 0.3)',
          ...aiGlow.thinking,
        }}
      >
        <View className="w-8 h-8 rounded-full mr-3 overflow-hidden">
          <Image
            source={require('../../../assets/pierre-logo.png')}
            className="w-8 h-8"
            resizeMode="cover"
          />
        </View>
        <View className="flex-row items-center gap-1">
          <View className="w-2 h-2 rounded-full bg-pierre-violet opacity-60" />
          <View className="w-2 h-2 rounded-full bg-pierre-violet opacity-80" />
          <View className="w-2 h-2 rounded-full bg-pierre-violet" />
        </View>
      </View>
    </View>
  );

  const renderCoachGridCard = (coach: Coach) => (
    <TouchableOpacity
      key={coach.id}
      className="bg-background-secondary rounded-xl p-4 w-[48%] border border-border-subtle mb-2"
      onPress={() => onCoachSelect(coach)}
      activeOpacity={0.7}
    >
      <View className="flex-row justify-between items-start mb-1 gap-2">
        <Text className="flex-1 text-sm font-semibold text-text-primary leading-[18px]" numberOfLines={2}>
          {coach.title}
        </Text>
        <View
          className="w-7 h-7 rounded items-center justify-center"
          style={{ backgroundColor: COACH_CATEGORY_BADGE_BG[coach.category] }}
        >
          <Text className="text-sm">
            {COACH_CATEGORY_ICONS[coach.category]}
          </Text>
        </View>
      </View>
      {coach.description && (
        <Text className="text-xs text-text-secondary leading-4 mb-1" numberOfLines={2}>
          {coach.description}
        </Text>
      )}
      <View className="flex-row items-center gap-2 mt-1">
        {coach.is_system && (
          <View className="px-2 py-0.5 rounded" style={{ backgroundColor: 'rgba(124, 58, 237, 0.15)' }}>
            <Text className="text-xs font-medium" style={{ color: '#7C3AED' }}>System</Text>
          </View>
        )}
        {coach.is_favorite && (
          <View className="px-1 py-0.5 rounded" style={{ backgroundColor: 'rgba(245, 158, 11, 0.15)' }}>
            <Text className="text-xs" style={{ color: '#F59E0B' }}>★</Text>
          </View>
        )}
        <View className="flex-1" />
        {coach.use_count > 0 && (
          <Text className="text-xs text-text-tertiary">{coach.use_count}×</Text>
        )}
      </View>
    </TouchableOpacity>
  );

  const renderEmptyChat = () => (
    <ScrollView
      className="flex-1"
      contentContainerStyle={{ flexGrow: 1, alignItems: 'center', justifyContent: 'flex-start', paddingHorizontal: spacing.xs, paddingVertical: spacing.md, paddingBottom: spacing.md }}
      showsVerticalScrollIndicator={false}
      keyboardShouldPersistTaps="handled"
    >
      {!isCoachConversation && coaches.length > 0 && (
        <View className="w-full px-1">
          <Text className="text-lg font-semibold text-text-primary mb-4">🎯 Your Coaches</Text>
          <View className="flex-row flex-wrap justify-between gap-2">
            {coaches.map((coach) => renderCoachGridCard(coach))}
          </View>
        </View>
      )}

      {!isCoachConversation && coaches.length === 0 && (
        <View className="flex-1 items-center justify-center px-8 py-12">
          <Text className="text-lg font-semibold text-text-primary mb-2">No coaches yet</Text>
          <Text className="text-base text-text-tertiary text-center">
            Create your first coach to customize how Pierre helps you.
          </Text>
        </View>
      )}

      {isCoachConversation && (
        <View className="w-full items-center px-4 mb-6">
          <Text className="text-base text-text-secondary text-center leading-6">
            Your coach is ready. Start the conversation by typing a message below.
          </Text>
        </View>
      )}
    </ScrollView>
  );

  if (isLoading) {
    return (
      <View className="flex-1 items-center justify-center">
        <ActivityIndicator size="large" color={colors.primary[500]} />
      </View>
    );
  }

  if ((messages?.length ?? 0) === 0 && !isSending) {
    return renderEmptyChat();
  }

  return (
    <FlatList
      ref={flatListRef}
      data={messages ?? []}
      renderItem={renderMessage}
      keyExtractor={(item, index) => item?.id ? `${item.id}-${index}` : `fallback-${index}`}
      contentContainerStyle={{ paddingHorizontal: spacing.md, paddingVertical: spacing.md, paddingBottom: spacing.md }}
      showsVerticalScrollIndicator={false}
      onContentSizeChange={onScrollToBottom}
      ListFooterComponent={isSending ? renderThinkingIndicator : null}
    />
  );
}
