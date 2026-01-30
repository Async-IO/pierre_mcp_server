// ABOUTME: Full-screen modal for previewing and sharing chat insights to social feed
// ABOUTME: Provides quick share option or navigation to full edit screen

import React from 'react';
import {
  View,
  Text,
  Modal,
  TouchableOpacity,
  ScrollView,
  ActivityIndicator,
} from 'react-native';
import { Ionicons, Feather } from '@expo/vector-icons';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, spacing } from '../../constants/theme';
import type { ShareVisibility } from '../../types';

interface SharePreviewModalProps {
  visible: boolean;
  content: string;
  visibility: ShareVisibility;
  isSharing: boolean;
  onVisibilityChange: (visibility: ShareVisibility) => void;
  onShare: () => void;
  onEdit: () => void;
  onClose: () => void;
}

export function SharePreviewModal({
  visible,
  content,
  visibility,
  isSharing,
  onVisibilityChange,
  onShare,
  onEdit,
  onClose,
}: SharePreviewModalProps) {
  const insets = useSafeAreaInsets();

  return (
    <Modal
      visible={visible}
      animationType="slide"
      presentationStyle="pageSheet"
      onRequestClose={onClose}
    >
      <View
        className="flex-1 bg-background-primary"
        style={{ paddingTop: insets.top }}
      >
        {/* Header */}
        <View className="flex-row items-center justify-between px-4 py-3 border-b border-border-subtle">
          <TouchableOpacity
            className="p-2"
            onPress={onClose}
            testID="close-share-preview"
          >
            <Feather name="x" size={24} color={colors.text.primary} />
          </TouchableOpacity>
          <Text className="text-lg font-bold text-text-primary">
            Share to Feed
          </Text>
          <TouchableOpacity
            className="p-2"
            onPress={onEdit}
            testID="edit-share-content"
          >
            <Feather name="edit-2" size={20} color={colors.pierre.violet} />
          </TouchableOpacity>
        </View>

        <ScrollView
          className="flex-1 px-4"
          showsVerticalScrollIndicator={false}
          contentContainerStyle={{ paddingBottom: 24 }}
        >
          {/* Insight Preview - constrained height to keep visibility options visible */}
          <View className="mt-6">
            <Text className="text-text-tertiary text-sm uppercase tracking-wide mb-2">
              Preview
            </Text>
            <View className="bg-background-secondary rounded-xl p-4 max-h-48">
              <View className="flex-row items-center mb-3">
                <View
                  className="w-8 h-8 rounded-full items-center justify-center mr-2"
                  style={{ backgroundColor: colors.pierre.violet + '20' }}
                >
                  <Feather name="cpu" size={16} color={colors.pierre.violet} />
                </View>
                <Text className="text-text-secondary text-sm">
                  Coach Insight
                </Text>
              </View>
              <ScrollView
                showsVerticalScrollIndicator={true}
                nestedScrollEnabled={true}
              >
                <Text className="text-text-primary text-base leading-6">
                  {content}
                </Text>
              </ScrollView>
            </View>
          </View>

          {/* Visibility Options */}
          <View className="mt-6">
            <Text className="text-text-tertiary text-sm uppercase tracking-wide mb-3">
              Who can see this?
            </Text>

            <TouchableOpacity
              className={`flex-row items-center rounded-xl p-4 mb-3 border-2 ${
                visibility === 'friends_only'
                  ? 'border-primary-500 bg-primary-500/10'
                  : 'border-border-default bg-background-secondary'
              }`}
              onPress={() => onVisibilityChange('friends_only')}
              testID="visibility-friends-only"
            >
              <View
                className="w-10 h-10 rounded-full items-center justify-center mr-3"
                style={{
                  backgroundColor:
                    visibility === 'friends_only'
                      ? colors.primary[500] + '20'
                      : colors.background.tertiary,
                }}
              >
                <Ionicons
                  name="people"
                  size={20}
                  color={
                    visibility === 'friends_only'
                      ? colors.primary[500]
                      : colors.text.secondary
                  }
                />
              </View>
              <View className="flex-1">
                <Text className="text-base font-semibold text-text-primary">
                  Friends Only
                </Text>
                <Text className="text-sm text-text-tertiary">
                  Only your friends can see this
                </Text>
              </View>
              {visibility === 'friends_only' && (
                <Ionicons
                  name="checkmark-circle"
                  size={24}
                  color={colors.primary[500]}
                />
              )}
            </TouchableOpacity>

            <TouchableOpacity
              className={`flex-row items-center rounded-xl p-4 border-2 ${
                visibility === 'public'
                  ? 'border-primary-500 bg-primary-500/10'
                  : 'border-border-default bg-background-secondary'
              }`}
              onPress={() => onVisibilityChange('public')}
              testID="visibility-public"
            >
              <View
                className="w-10 h-10 rounded-full items-center justify-center mr-3"
                style={{
                  backgroundColor:
                    visibility === 'public'
                      ? colors.primary[500] + '20'
                      : colors.background.tertiary,
                }}
              >
                <Ionicons
                  name="globe"
                  size={20}
                  color={
                    visibility === 'public'
                      ? colors.primary[500]
                      : colors.text.secondary
                  }
                />
              </View>
              <View className="flex-1">
                <Text className="text-base font-semibold text-text-primary">
                  Public
                </Text>
                <Text className="text-sm text-text-tertiary">
                  Anyone can see this
                </Text>
              </View>
              {visibility === 'public' && (
                <Ionicons
                  name="checkmark-circle"
                  size={24}
                  color={colors.primary[500]}
                />
              )}
            </TouchableOpacity>
          </View>

          {/* Privacy Notice */}
          <View
            className="flex-row items-start rounded-xl p-4 mt-6"
            style={{ backgroundColor: colors.pierre.violet + '15' }}
          >
            <Feather
              name="shield"
              size={18}
              color={colors.pierre.violet}
              style={{ marginRight: spacing.sm, marginTop: 2 }}
            />
            <Text className="flex-1 text-text-secondary text-sm leading-5">
              Your privacy matters. Personal data like exact locations and health
              metrics are automatically removed before sharing.
            </Text>
          </View>
        </ScrollView>

        {/* Footer Actions */}
        <View
          className="px-4 pt-4 border-t border-border-subtle"
          style={{ paddingBottom: Math.max(insets.bottom, 16) }}
        >
          <TouchableOpacity
            className="rounded-xl py-4 items-center"
            style={{
              backgroundColor: isSharing
                ? colors.primary[400]
                : colors.primary[600],
            }}
            onPress={onShare}
            disabled={isSharing}
            testID="share-now-button"
          >
            {isSharing ? (
              <ActivityIndicator size="small" color={colors.text.primary} />
            ) : (
              <Text className="text-base font-bold text-text-primary">
                Share Now
              </Text>
            )}
          </TouchableOpacity>

          <TouchableOpacity
            className="py-3 items-center mt-2"
            onPress={onClose}
            testID="cancel-share-button"
          >
            <Text className="text-base text-text-tertiary">Cancel</Text>
          </TouchableOpacity>
        </View>
      </View>
    </Modal>
  );
}
