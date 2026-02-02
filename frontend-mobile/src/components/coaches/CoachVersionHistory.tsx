// ABOUTME: Coach version history modal for viewing and reverting to previous versions
// ABOUTME: Displays timeline of versions with expand/collapse and revert functionality

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  Modal,
  ScrollView,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFocusEffect } from '@react-navigation/native';
import { Ionicons } from '@expo/vector-icons';
import { colors, spacing, glassCard } from '../../constants/theme';
import { coachesApi } from '../../services/api';

// Shadow style for version cards
const versionCardShadow: ViewStyle = {
  shadowColor: glassCard.shadowColor,
  shadowOffset: glassCard.shadowOffset,
  shadowOpacity: glassCard.shadowOpacity,
  shadowRadius: glassCard.shadowRadius,
  elevation: glassCard.elevation,
};

interface CoachVersionHistoryProps {
  coachId: string;
  coachTitle: string;
  isOpen: boolean;
  onClose: () => void;
  onReverted?: () => void;
}

interface VersionItem {
  version: number;
  content_snapshot: Record<string, unknown>;
  change_summary: string | null;
  created_at: string;
  created_by_name: string | null;
}

interface VersionsResponse {
  versions: VersionItem[];
  total: number;
  current_version: number;
}

export function CoachVersionHistory({
  coachId,
  coachTitle,
  isOpen,
  onClose,
  onReverted,
}: CoachVersionHistoryProps) {
  const insets = useSafeAreaInsets();
  const [versionsData, setVersionsData] = useState<VersionsResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isReverting, setIsReverting] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<VersionItem | null>(null);

  const loadVersions = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await coachesApi.getVersions(coachId, 50);
      setVersionsData(response);
    } catch (error) {
      console.error('Failed to load versions:', error);
      Alert.alert('Error', 'Failed to load version history');
    } finally {
      setIsLoading(false);
    }
  }, [coachId]);

  // Load versions when modal opens
  useFocusEffect(
    useCallback(() => {
      if (isOpen) {
        loadVersions();
      }
    }, [isOpen, loadVersions])
  );

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const handleVersionPress = (version: VersionItem) => {
    if (selectedVersion?.version === version.version) {
      setSelectedVersion(null);
    } else {
      setSelectedVersion(version);
    }
  };

  const handleRevert = (version: VersionItem) => {
    Alert.alert(
      'Confirm Revert',
      `Are you sure you want to revert to version ${version.version}? This will create a new version with the reverted content. Your current changes will be preserved in the version history.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Revert',
          style: 'destructive',
          onPress: () => confirmRevert(version.version),
        },
      ]
    );
  };

  const confirmRevert = async (versionNumber: number) => {
    try {
      setIsReverting(true);
      await coachesApi.revertToVersion(coachId, versionNumber);
      Alert.alert('Success', 'Coach reverted successfully');
      setSelectedVersion(null);
      onReverted?.();
      onClose();
    } catch (error) {
      console.error('Failed to revert:', error);
      Alert.alert('Error', 'Failed to revert to version');
    } finally {
      setIsReverting(false);
    }
  };

  const renderSnapshotField = (key: string, value: unknown) => {
    if (value === null || value === undefined) return null;

    const displayValue =
      typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);

    // Truncate very long values
    const truncatedValue =
      displayValue.length > 200 ? `${displayValue.substring(0, 200)}...` : displayValue;

    return (
      <View key={key} className="py-2 border-b border-border-subtle/30 last:border-0">
        <Text className="text-xs font-medium text-text-tertiary uppercase tracking-wide mb-1">
          {key.replace(/_/g, ' ')}
        </Text>
        <Text className="text-sm text-text-primary" numberOfLines={5}>
          {truncatedValue}
        </Text>
      </View>
    );
  };

  const renderVersionItem = (version: VersionItem) => {
    const isSelected = selectedVersion?.version === version.version;
    const isCurrent = versionsData?.current_version === version.version;

    return (
      <View
        key={version.version}
        className={`rounded-lg mb-3 border ${
          isSelected ? 'border-primary-500 bg-primary-500/10' : 'border-border-subtle bg-background-secondary'
        }`}
        style={versionCardShadow}
      >
        {/* Version Header */}
        <TouchableOpacity
          className="flex-row items-center p-3"
          onPress={() => handleVersionPress(version)}
          testID={`version-item-${version.version}`}
        >
          <View className="w-10 h-10 rounded-full bg-background-tertiary items-center justify-center mr-3">
            <Text className="text-sm font-semibold text-text-primary">v{version.version}</Text>
          </View>
          <View className="flex-1">
            <View className="flex-row items-center gap-2">
              <Text className="text-base font-medium text-text-primary">
                {version.change_summary || 'Update'}
              </Text>
              {isCurrent && (
                <View className="px-2 py-0.5 rounded bg-primary-500/20">
                  <Text className="text-xs font-medium text-primary-400">Current</Text>
                </View>
              )}
            </View>
            <Text className="text-xs text-text-tertiary mt-0.5">
              {formatDate(version.created_at)}
              {version.created_by_name && ` by ${version.created_by_name}`}
            </Text>
          </View>
          <Ionicons
            name={isSelected ? 'chevron-up' : 'chevron-down'}
            size={20}
            color={colors.text.tertiary}
          />
        </TouchableOpacity>

        {/* Expanded Content */}
        {isSelected && (
          <View className="px-3 pb-3 border-t border-border-subtle/30">
            {/* Snapshot Content */}
            <View className="bg-background-primary/50 rounded-lg p-3 mt-3">
              <Text className="text-xs font-semibold text-text-secondary uppercase tracking-wide mb-2">
                Snapshot Content
              </Text>
              {Object.entries(version.content_snapshot).map(([key, value]) =>
                renderSnapshotField(key, value)
              )}
            </View>

            {/* Revert Button */}
            {!isCurrent && (
              <TouchableOpacity
                className={`mt-3 py-2 px-4 rounded-lg border border-warning self-end ${
                  isReverting ? 'opacity-50' : ''
                }`}
                onPress={() => handleRevert(version)}
                disabled={isReverting}
                testID={`revert-button-${version.version}`}
              >
                {isReverting ? (
                  <ActivityIndicator size="small" color={colors.warning} />
                ) : (
                  <Text className="text-sm font-medium text-warning">
                    Revert to v{version.version}
                  </Text>
                )}
              </TouchableOpacity>
            )}
          </View>
        )}
      </View>
    );
  };

  return (
    <Modal
      visible={isOpen}
      animationType="slide"
      transparent
      onRequestClose={onClose}
    >
      <View className="flex-1 bg-black/50 justify-end">
        <View
          className="bg-background-primary rounded-t-2xl max-h-[85%]"
          style={{ paddingBottom: insets.bottom + spacing.md }}
        >
          {/* Header */}
          <View className="flex-row items-center justify-between px-4 py-3 border-b border-border-subtle">
            <View className="flex-1">
              <Text className="text-lg font-semibold text-text-primary" numberOfLines={1}>
                Version History
              </Text>
              <Text className="text-sm text-text-secondary" numberOfLines={1}>
                {coachTitle}
              </Text>
            </View>
            <TouchableOpacity
              className="p-2 -mr-2"
              onPress={onClose}
              testID="close-version-history"
            >
              <Ionicons name="close" size={24} color={colors.text.secondary} />
            </TouchableOpacity>
          </View>

          {/* Stats Bar */}
          {versionsData && (
            <View className="flex-row items-center justify-between px-4 py-2 bg-background-secondary/50">
              <Text className="text-sm text-text-secondary">
                {versionsData.total} version{versionsData.total !== 1 ? 's' : ''} saved
              </Text>
              <Text className="text-sm font-medium text-text-primary">
                Current: v{versionsData.current_version}
              </Text>
            </View>
          )}

          {/* Content */}
          {isLoading ? (
            <View className="flex-1 items-center justify-center py-12">
              <ActivityIndicator size="large" color={colors.primary[500]} />
              <Text className="mt-3 text-text-secondary">Loading versions...</Text>
            </View>
          ) : !versionsData || versionsData.versions.length === 0 ? (
            <View className="flex-1 items-center justify-center py-12 px-6">
              <Ionicons name="git-branch-outline" size={48} color={colors.text.tertiary} />
              <Text className="text-base text-text-secondary text-center mt-3">
                No version history yet
              </Text>
              <Text className="text-sm text-text-tertiary text-center mt-1">
                Versions are created automatically when you update the coach.
              </Text>
            </View>
          ) : (
            <ScrollView
              className="flex-1 px-4 pt-3"
              showsVerticalScrollIndicator={false}
              testID="version-list"
            >
              {versionsData.versions.map(renderVersionItem)}
              <View className="h-4" />
            </ScrollView>
          )}
        </View>
      </View>
    </Modal>
  );
}

export default CoachVersionHistory;
