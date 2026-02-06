// ABOUTME: Friend card component for displaying friend information
// ABOUTME: Shows avatar, name, status, and action buttons

import React from 'react';
import { View, Text, TouchableOpacity, ActivityIndicator, type ViewStyle } from 'react-native';
import { Feather } from '@expo/vector-icons';
import { colors, glassCard, buttonGlow } from '../../constants/theme';
import type { FriendWithInfo, DiscoverableUser, PendingRequestWithInfo } from '@pierre/shared-types';

// Generate avatar initials from name or email
const getInitials = (name: string | null, email?: string): string => {
  if (name) {
    const parts = name.split(' ');
    if (parts.length >= 2) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return name.substring(0, 2).toUpperCase();
  }
  if (email) {
    return email.substring(0, 2).toUpperCase();
  }
  return '??';
};

// Generate consistent color from string
const getAvatarColor = (str: string): string => {
  const hash = str.split('').reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 70%, 50%)`;
};

// Glass card style with violet accent border
const cardStyle: ViewStyle = {
  ...glassCard,
  borderRadius: 12,
  borderColor: 'rgba(139, 92, 246, 0.15)',
};

interface FriendCardProps {
  friend: FriendWithInfo;
  onRemove?: () => void;
  isRemoving?: boolean;
}

export function FriendCard({ friend, onRemove, isRemoving }: FriendCardProps) {
  const displayName = friend.friend_display_name || friend.friend_email;
  const initials = getInitials(friend.friend_display_name, friend.friend_email);
  const avatarColor = getAvatarColor(friend.friend_email);

  return (
    <View
      className="flex-row items-center p-3 mx-3 my-1 rounded-lg"
      style={cardStyle}
    >
      <View
        className="w-12 h-12 rounded-full justify-center items-center"
        style={{ backgroundColor: avatarColor }}
      >
        <Text className="text-lg font-semibold text-text-primary">{initials}</Text>
      </View>
      <View className="flex-1 ml-3">
        <Text className="text-base font-semibold text-text-primary" numberOfLines={1}>
          {displayName}
        </Text>
        {friend.accepted_at && (
          <Text className="text-sm text-text-secondary mt-0.5">
            Friends since {new Date(friend.accepted_at).toLocaleDateString()}
          </Text>
        )}
      </View>
      {onRemove && (
        <TouchableOpacity
          className="p-2"
          onPress={onRemove}
          disabled={isRemoving}
        >
          {isRemoving ? (
            <ActivityIndicator size="small" color={colors.text.secondary} />
          ) : (
            <Feather name="user-minus" size={20} color={colors.text.secondary} />
          )}
        </TouchableOpacity>
      )}
    </View>
  );
}

interface RequestCardProps {
  request: PendingRequestWithInfo;
  type: 'incoming' | 'outgoing';
  onAccept?: () => void;
  onDecline?: () => void;
  onCancel?: () => void;
  isLoading?: boolean;
}

export function RequestCard({
  request,
  type,
  onAccept,
  onDecline,
  onCancel,
  isLoading,
}: RequestCardProps) {
  // PendingRequestWithInfo includes user_display_name, user_email, user_id
  const userEmail = request.user_email;
  const initials = getInitials(request.user_display_name, userEmail);
  const avatarColor = getAvatarColor(userEmail);
  const name = request.user_display_name || userEmail;

  return (
    <View
      className="flex-row items-center p-3 mx-3 my-1 rounded-lg"
      style={cardStyle}
    >
      <View
        className="w-12 h-12 rounded-full justify-center items-center"
        style={{ backgroundColor: avatarColor }}
      >
        <Text className="text-lg font-semibold text-text-primary">{initials}</Text>
      </View>
      <View className="flex-1 ml-3">
        <Text className="text-base font-semibold text-text-primary" numberOfLines={1}>
          {name}
        </Text>
        <Text className="text-sm text-text-secondary mt-0.5">
          {type === 'incoming' ? 'Wants to be friends' : 'Request sent'} •{' '}
          {new Date(request.created_at).toLocaleDateString()}
        </Text>
      </View>
      {isLoading ? (
        <ActivityIndicator size="small" color={colors.pierre.violet} />
      ) : type === 'incoming' ? (
        <View className="flex-row gap-2">
          <TouchableOpacity
            className="w-10 h-10 rounded-full justify-center items-center"
            style={{
              backgroundColor: colors.pierre.activity,
              shadowColor: colors.pierre.activity,
              shadowOffset: { width: 0, height: 0 },
              shadowOpacity: 0.4,
              shadowRadius: 8,
              elevation: 4,
            }}
            onPress={onAccept}
          >
            <Feather name="check" size={18} color="#FFFFFF" />
          </TouchableOpacity>
          <TouchableOpacity
            className="w-10 h-10 rounded-full justify-center items-center"
            style={{ ...glassCard, borderRadius: 20 }}
            onPress={onDecline}
          >
            <Feather name="x" size={18} color={colors.text.secondary} />
          </TouchableOpacity>
        </View>
      ) : (
        <TouchableOpacity
          className="px-3 py-2 rounded-lg bg-background-tertiary"
          onPress={onCancel}
        >
          <Text className="text-sm text-text-secondary">Cancel</Text>
        </TouchableOpacity>
      )}
    </View>
  );
}

interface SearchUserCardProps {
  user: DiscoverableUser;
  onAddFriend: () => void;
  isAdding?: boolean;
}

export function SearchUserCard({ user, onAddFriend, isAdding }: SearchUserCardProps) {
  const initials = getInitials(user.display_name, user.email);
  const avatarColor = getAvatarColor(user.email ?? user.id);
  const displayName = user.display_name || user.email || 'Unknown User';

  return (
    <View
      className="flex-row items-center p-3 mx-3 my-1 rounded-lg"
      style={cardStyle}
    >
      <View
        className="w-12 h-12 rounded-full justify-center items-center"
        style={{ backgroundColor: avatarColor }}
      >
        <Text className="text-lg font-semibold text-text-primary">{initials}</Text>
      </View>
      <View className="flex-1 ml-3">
        <Text className="text-base font-semibold text-text-primary" numberOfLines={1}>
          {displayName}
        </Text>
      </View>
      {user.is_friend ? (
        <View className="flex-row items-center px-3 py-2 gap-1">
          <Feather name="check" size={14} color={colors.pierre.activity} />
          <Text className="text-sm" style={{ color: colors.pierre.activity }}>Friends</Text>
        </View>
      ) : user.has_pending_request ? (
        <View className="px-3 py-2 rounded-lg bg-background-tertiary">
          <Text className="text-sm text-text-tertiary">Pending</Text>
        </View>
      ) : (
        <TouchableOpacity
          className="flex-row items-center px-4 py-2.5 rounded-xl gap-1.5"
          style={{
            backgroundColor: colors.pierre.violet,
            ...buttonGlow,
          }}
          onPress={onAddFriend}
          disabled={isAdding}
        >
          {isAdding ? (
            <ActivityIndicator size="small" color="#FFFFFF" />
          ) : (
            <>
              <Feather name="user-plus" size={16} color="#FFFFFF" />
              <Text className="text-sm font-semibold text-white">Add</Text>
            </>
          )}
        </TouchableOpacity>
      )}
    </View>
  );
}
