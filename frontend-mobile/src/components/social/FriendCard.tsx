// ABOUTME: Friend card component for displaying friend information
// ABOUTME: Shows avatar, name, status, and action buttons

import React from 'react';
import { View, Text, StyleSheet, TouchableOpacity, ActivityIndicator } from 'react-native';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import type { FriendWithInfo, FriendConnection, DiscoverableUser } from '../../types';

// Generate avatar initials from name or email
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

// Generate consistent color from string
const getAvatarColor = (str: string): string => {
  const hash = str.split('').reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 70%, 50%)`;
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
    <View style={styles.card}>
      <View style={[styles.avatar, { backgroundColor: avatarColor }]}>
        <Text style={styles.avatarText}>{initials}</Text>
      </View>
      <View style={styles.info}>
        <Text style={styles.name} numberOfLines={1}>
          {displayName}
        </Text>
        {friend.accepted_at && (
          <Text style={styles.connectedSince}>
            Friends since {new Date(friend.accepted_at).toLocaleDateString()}
          </Text>
        )}
      </View>
      {onRemove && (
        <TouchableOpacity
          style={styles.removeButton}
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
  request: FriendConnection;
  type: 'incoming' | 'outgoing';
  displayName?: string | null;
  email?: string;
  onAccept?: () => void;
  onDecline?: () => void;
  onCancel?: () => void;
  isLoading?: boolean;
}

export function RequestCard({
  request,
  type,
  displayName,
  email,
  onAccept,
  onDecline,
  onCancel,
  isLoading,
}: RequestCardProps) {
  const userEmail = email || (type === 'incoming' ? request.initiator_id : request.receiver_id);
  const initials = getInitials(displayName || null, userEmail);
  const avatarColor = getAvatarColor(userEmail);
  const name = displayName || userEmail;

  return (
    <View style={styles.card}>
      <View style={[styles.avatar, { backgroundColor: avatarColor }]}>
        <Text style={styles.avatarText}>{initials}</Text>
      </View>
      <View style={styles.info}>
        <Text style={styles.name} numberOfLines={1}>
          {name}
        </Text>
        <Text style={styles.requestDate}>
          {type === 'incoming' ? 'Wants to be friends' : 'Request sent'} •{' '}
          {new Date(request.created_at).toLocaleDateString()}
        </Text>
      </View>
      {isLoading ? (
        <ActivityIndicator size="small" color={colors.pierre.violet} />
      ) : type === 'incoming' ? (
        <View style={styles.actionButtons}>
          <TouchableOpacity
            style={[styles.actionButton, styles.acceptButton]}
            onPress={onAccept}
          >
            <Feather name="check" size={18} color={colors.text.primary} />
          </TouchableOpacity>
          <TouchableOpacity
            style={[styles.actionButton, styles.declineButton]}
            onPress={onDecline}
          >
            <Feather name="x" size={18} color={colors.text.secondary} />
          </TouchableOpacity>
        </View>
      ) : (
        <TouchableOpacity style={styles.cancelButton} onPress={onCancel}>
          <Text style={styles.cancelText}>Cancel</Text>
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
  const avatarColor = getAvatarColor(user.email);
  const displayName = user.display_name || user.email;

  return (
    <View style={styles.card}>
      <View style={[styles.avatar, { backgroundColor: avatarColor }]}>
        <Text style={styles.avatarText}>{initials}</Text>
      </View>
      <View style={styles.info}>
        <Text style={styles.name} numberOfLines={1}>
          {displayName}
        </Text>
        {user.mutual_friends_count > 0 && (
          <Text style={styles.mutualFriends}>
            {user.mutual_friends_count} mutual friend{user.mutual_friends_count > 1 ? 's' : ''}
          </Text>
        )}
      </View>
      {user.is_friend ? (
        <View style={styles.friendBadge}>
          <Feather name="check" size={14} color={colors.pierre.activity} />
          <Text style={styles.friendBadgeText}>Friends</Text>
        </View>
      ) : user.pending_request ? (
        <View style={styles.pendingBadge}>
          <Text style={styles.pendingText}>Pending</Text>
        </View>
      ) : (
        <TouchableOpacity
          style={styles.addButton}
          onPress={onAddFriend}
          disabled={isAdding}
        >
          {isAdding ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <>
              <Feather name="user-plus" size={16} color={colors.text.primary} />
              <Text style={styles.addButtonText}>Add</Text>
            </>
          )}
        </TouchableOpacity>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: spacing.md,
    marginHorizontal: spacing.md,
    marginVertical: spacing.xs,
    borderRadius: borderRadius.lg,
    ...glassCard,
  },
  avatar: {
    width: 48,
    height: 48,
    borderRadius: 24,
    justifyContent: 'center',
    alignItems: 'center',
  },
  avatarText: {
    color: colors.text.primary,
    fontSize: fontSize.lg,
    fontWeight: '600',
  },
  info: {
    flex: 1,
    marginLeft: spacing.md,
  },
  name: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  connectedSince: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginTop: 2,
  },
  requestDate: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    marginTop: 2,
  },
  mutualFriends: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    marginTop: 2,
  },
  removeButton: {
    padding: spacing.sm,
  },
  actionButtons: {
    flexDirection: 'row',
    gap: spacing.sm,
  },
  actionButton: {
    width: 36,
    height: 36,
    borderRadius: 18,
    justifyContent: 'center',
    alignItems: 'center',
  },
  acceptButton: {
    backgroundColor: colors.pierre.activity,
  },
  declineButton: {
    backgroundColor: colors.background.tertiary,
  },
  cancelButton: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.background.tertiary,
  },
  cancelText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
  },
  addButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.pierre.violet,
    gap: spacing.xs,
  },
  addButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
  friendBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    gap: spacing.xs,
  },
  friendBadgeText: {
    color: colors.pierre.activity,
    fontSize: fontSize.sm,
  },
  pendingBadge: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    backgroundColor: colors.background.tertiary,
  },
  pendingText: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
  },
});
