// ABOUTME: Coach version history modal for viewing and reverting to previous versions
// ABOUTME: Displays timeline of versions with change summaries and revert functionality

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  Modal,
  TouchableOpacity,
  ScrollView,
  ActivityIndicator,
  Alert,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';
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
      <View key={key} style={styles.snapshotField}>
        <Text style={styles.snapshotLabel}>{key.replace(/_/g, ' ')}</Text>
        <Text style={styles.snapshotValue}>{displayValue}</Text>
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
      <View style={[styles.container, { paddingTop: insets.top }]}>
        {/* Header */}
        <View style={styles.header}>
          <TouchableOpacity onPress={onClose} style={styles.closeButton}>
            <Text style={styles.closeButtonText}>Close</Text>
          </TouchableOpacity>
          <Text style={styles.title} numberOfLines={1}>
            History: {coachTitle}
          </Text>
          <View style={styles.headerSpacer} />
        </View>

        {/* Stats bar */}
        <View style={styles.statsBar}>
          <Text style={styles.statsText}>
            {versions.length} version{versions.length !== 1 ? 's' : ''} saved
          </Text>
          <Text style={styles.statsTextBold}>Current: v{currentVersion}</Text>
        </View>

        {/* Content */}
        {isLoading ? (
          <View style={styles.loadingContainer}>
            <ActivityIndicator size="large" color={colors.primary[500]} />
            <Text style={styles.loadingText}>Loading versions...</Text>
          </View>
        ) : versions.length === 0 ? (
          <View style={styles.emptyContainer}>
            <Text style={styles.emptyText}>No version history yet.</Text>
            <Text style={styles.emptySubtext}>
              Versions are created automatically when you update the coach.
            </Text>
          </View>
        ) : (
          <ScrollView style={styles.versionList} contentContainerStyle={styles.versionListContent}>
            {versions.map((version) => (
              <View key={version.version} style={styles.versionItem}>
                {/* Version header */}
                <TouchableOpacity
                  onPress={() => handleVersionPress(version)}
                  style={[
                    styles.versionHeader,
                    selectedVersion?.version === version.version && styles.versionHeaderSelected,
                  ]}
                >
                  <View style={styles.versionBadge}>
                    <Text style={styles.versionBadgeText}>v{version.version}</Text>
                  </View>
                  <View style={styles.versionInfo}>
                    <Text style={styles.versionSummary} numberOfLines={1}>
                      {version.change_summary || 'Update'}
                    </Text>
                    <Text style={styles.versionDate}>
                      {formatDate(version.created_at)}
                      {version.created_by_name && ` by ${version.created_by_name}`}
                    </Text>
                  </View>
                  <Text style={styles.expandIcon}>
                    {selectedVersion?.version === version.version ? '▼' : '▶'}
                  </Text>
                </TouchableOpacity>

                {/* Expanded content */}
                {selectedVersion?.version === version.version && (
                  <View style={styles.versionContent}>
                    <Text style={styles.snapshotTitle}>Snapshot Content</Text>
                    <View style={styles.snapshotContainer}>
                      {Object.entries(version.content_snapshot).map(([key, value]) =>
                        renderSnapshotField(key, value)
                      )}
                    </View>
                    <TouchableOpacity
                      onPress={handleRevert}
                      style={[styles.revertButton, isReverting && styles.revertButtonDisabled]}
                      disabled={isReverting}
                    >
                      {isReverting ? (
                        <ActivityIndicator size="small" color={colors.text.primary} />
                      ) : (
                        <Text style={styles.revertButtonText}>
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

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  closeButton: {
    padding: spacing.sm,
  },
  closeButtonText: {
    color: colors.primary[500],
    fontSize: fontSize.md,
  },
  title: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  headerSpacer: {
    width: 60,
  },
  statsBar: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    backgroundColor: colors.background.tertiary,
  },
  statsText: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  statsTextBold: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.primary,
  },
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  loadingText: {
    marginTop: spacing.md,
    fontSize: fontSize.md,
    color: colors.text.secondary,
  },
  emptyContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: spacing.xl,
  },
  emptyText: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.sm,
  },
  emptySubtext: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
  },
  versionList: {
    flex: 1,
  },
  versionListContent: {
    padding: spacing.md,
  },
  versionItem: {
    marginBottom: spacing.sm,
    borderRadius: borderRadius.md,
    borderWidth: 1,
    borderColor: colors.border.default,
    backgroundColor: colors.background.secondary,
    overflow: 'hidden',
  },
  versionHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: spacing.md,
  },
  versionHeaderSelected: {
    backgroundColor: colors.background.tertiary,
  },
  versionBadge: {
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: colors.background.tertiary,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: spacing.md,
  },
  versionBadgeText: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.primary,
  },
  versionInfo: {
    flex: 1,
  },
  versionSummary: {
    fontSize: fontSize.md,
    fontWeight: '500',
    color: colors.text.primary,
    marginBottom: 2,
  },
  versionDate: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  expandIcon: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    marginLeft: spacing.sm,
  },
  versionContent: {
    padding: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.default,
    backgroundColor: colors.background.tertiary,
  },
  snapshotTitle: {
    fontSize: fontSize.xs,
    fontWeight: '600',
    color: colors.text.secondary,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: spacing.sm,
  },
  snapshotContainer: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.sm,
    padding: spacing.sm,
    marginBottom: spacing.md,
  },
  snapshotField: {
    paddingVertical: spacing.xs,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.default,
  },
  snapshotLabel: {
    fontSize: fontSize.xs,
    fontWeight: '500',
    color: colors.text.secondary,
    textTransform: 'capitalize',
    marginBottom: 2,
  },
  snapshotValue: {
    fontSize: fontSize.sm,
    color: colors.text.primary,
  },
  revertButton: {
    backgroundColor: colors.primary[500],
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.md,
    borderRadius: borderRadius.sm,
    alignItems: 'center',
    alignSelf: 'flex-end',
  },
  revertButtonDisabled: {
    opacity: 0.6,
  },
  revertButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
});

export default CoachVersionHistoryModal;
