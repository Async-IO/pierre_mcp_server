// ABOUTME: Coach version history modal for viewing and reverting to previous versions
// ABOUTME: Displays timeline of versions with change summaries and revert functionality

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  Modal,
  TouchableOpacity,
  ScrollView,
  ActivityIndicator,
  Alert,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, spacing } from '../constants/theme';
import { apiService } from '../services/api';

interface VersionItem {
  version: number;
  content_snapshot: Record<string, unknown>;
  change_summary: string | null;
  created_at: string;
  created_by_name: string | null;
}

interface CoachVersionHistoryModalProps {
  visible: boolean;
  onClose: () => void;
  coachId: string;
  coachTitle: string;
  onReverted?: () => void;
}

export function CoachVersionHistoryModal({
  visible,
  onClose,
  coachId,
  coachTitle,
  onReverted,
}: CoachVersionHistoryModalProps) {
  const insets = useSafeAreaInsets();
  const [versions, setVersions] = useState<VersionItem[]>([]);
  const [currentVersion, setCurrentVersion] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedVersion, setSelectedVersion] = useState<VersionItem | null>(null);
  const [isReverting, setIsReverting] = useState(false);

  const loadVersions = useCallback(async () => {
    try {
      setIsLoading(true);
      const response = await apiService.getCoachVersions(coachId, 50);
      setVersions(response.versions);
      setCurrentVersion(response.current_version);
    } catch (error) {
      console.error('Failed to load versions:', error);
      Alert.alert('Error', 'Failed to load version history');
    } finally {
      setIsLoading(false);
    }
  }, [coachId]);

  useEffect(() => {
    if (visible) {
      loadVersions();
    }
  }, [visible, loadVersions]);

  const formatDate = (dateString: string) => {
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

  const handleRevert = () => {
    if (!selectedVersion) return;

    Alert.alert(
      'Confirm Revert',
      `Are you sure you want to revert to version ${selectedVersion.version}? This will create a new version with the reverted content.`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Revert',
          style: 'destructive',
          onPress: confirmRevert,
        },
      ]
    );
  };

  const confirmRevert = async () => {
    if (!selectedVersion) return;

    try {
      setIsReverting(true);
      await apiService.revertCoachToVersion(coachId, selectedVersion.version);
      Alert.alert('Success', `Reverted to version ${selectedVersion.version}`);
      setSelectedVersion(null);
      onReverted?.();
      loadVersions();
    } catch (error) {
      console.error('Failed to revert:', error);
      Alert.alert('Error', 'Failed to revert to selected version');
    } finally {
      setIsReverting(false);
    }
  };

  const renderSnapshotField = (key: string, value: unknown) => {
    if (value === null || value === undefined) return null;

    const displayValue =
      typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);

    return (
      <View key={key} className="py-1 border-b border-border-default">
        <Text className="text-xs font-medium text-text-secondary capitalize mb-0.5">
          {key.replace(/_/g, ' ')}
        </Text>
        <Text className="text-sm text-text-primary">{displayValue}</Text>
      </View>
    );
  };

  return (
    <Modal
      visible={visible}
      animationType="slide"
      presentationStyle="pageSheet"
      onRequestClose={onClose}
    >
      <View className="flex-1 bg-background-primary" style={{ paddingTop: insets.top }}>
        {/* Header */}
        <View className="flex-row items-center justify-between px-3 py-2 border-b border-border-default">
          <TouchableOpacity onPress={onClose} className="p-2">
            <Text className="text-base text-primary-500">Close</Text>
          </TouchableOpacity>
          <Text className="flex-1 text-lg font-semibold text-text-primary text-center" numberOfLines={1}>
            History: {coachTitle}
          </Text>
          <View className="w-[60px]" />
        </View>

        {/* Stats bar */}
        <View className="flex-row justify-between items-center px-3 py-2 bg-background-tertiary">
          <Text className="text-sm text-text-secondary">
            {versions.length} version{versions.length !== 1 ? 's' : ''} saved
          </Text>
          <Text className="text-sm font-semibold text-text-primary">Current: v{currentVersion}</Text>
        </View>

        {/* Content */}
        {isLoading ? (
          <View className="flex-1 justify-center items-center">
            <ActivityIndicator size="large" color={colors.primary[500]} />
            <Text className="mt-3 text-base text-text-secondary">Loading versions...</Text>
          </View>
        ) : versions.length === 0 ? (
          <View className="flex-1 justify-center items-center px-6">
            <Text className="text-lg font-semibold text-text-primary mb-2">No version history yet.</Text>
            <Text className="text-base text-text-secondary text-center">
              Versions are created automatically when you update the coach.
            </Text>
          </View>
        ) : (
          <ScrollView className="flex-1" contentContainerStyle={{ padding: spacing.md }}>
            {versions.map((version) => (
              <View
                key={version.version}
                className="mb-2 rounded-lg border border-border-default bg-background-secondary overflow-hidden"
              >
                {/* Version header */}
                <TouchableOpacity
                  onPress={() => handleVersionPress(version)}
                  className={`flex-row items-center p-3 ${
                    selectedVersion?.version === version.version ? 'bg-background-tertiary' : ''
                  }`}
                >
                  <View className="w-9 h-9 rounded-full bg-background-tertiary justify-center items-center mr-3">
                    <Text className="text-sm font-semibold text-text-primary">v{version.version}</Text>
                  </View>
                  <View className="flex-1">
                    <Text className="text-base font-medium text-text-primary mb-0.5" numberOfLines={1}>
                      {version.change_summary || 'Update'}
                    </Text>
                    <Text className="text-sm text-text-secondary">
                      {formatDate(version.created_at)}
                      {version.created_by_name && ` by ${version.created_by_name}`}
                    </Text>
                  </View>
                  <Text className="text-sm text-text-secondary ml-2">
                    {selectedVersion?.version === version.version ? '▼' : '▶'}
                  </Text>
                </TouchableOpacity>

                {/* Expanded content */}
                {selectedVersion?.version === version.version && (
                  <View className="p-3 border-t border-border-default bg-background-tertiary">
                    <Text className="text-xs font-semibold text-text-secondary uppercase tracking-wide mb-2">
                      Snapshot Content
                    </Text>
                    <View className="bg-background-secondary rounded p-2 mb-3">
                      {Object.entries(version.content_snapshot).map(([key, value]) =>
                        renderSnapshotField(key, value)
                      )}
                    </View>
                    <TouchableOpacity
                      onPress={handleRevert}
                      className={`bg-primary-500 py-2 px-3 rounded items-center self-end ${
                        isReverting ? 'opacity-60' : ''
                      }`}
                      disabled={isReverting}
                    >
                      {isReverting ? (
                        <ActivityIndicator size="small" color={colors.text.primary} />
                      ) : (
                        <Text className="text-text-primary text-sm font-semibold">
                          Revert to v{version.version}
                        </Text>
                      )}
                    </TouchableOpacity>
                  </View>
                )}
              </View>
            ))}
          </ScrollView>
        )}
      </View>
    </Modal>
  );
}

export default CoachVersionHistoryModal;
