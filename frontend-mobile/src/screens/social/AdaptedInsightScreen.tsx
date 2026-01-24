// ABOUTME: Screen displaying the result of "Adapt to My Training" feature
// ABOUTME: Shows personalized coach insight adapted from friend's shared content

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  ScrollView,
  TouchableOpacity,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { Feather } from '@expo/vector-icons';
import { colors, spacing, fontSize, borderRadius, glassCard } from '../../constants/theme';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

type NavigationProp = DrawerNavigationProp<AppDrawerParamList>;
type RouteProps = RouteProp<AppDrawerParamList, 'AdaptedInsight'>;

// Format date for display
const formatDate = (dateStr: string): string => {
  const date = new Date(dateStr);
  return date.toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
};

export function AdaptedInsightScreen() {
  const navigation = useNavigation<NavigationProp>();
  const route = useRoute<RouteProps>();
  const { adaptedInsight } = route.params;

  return (
    <SafeAreaView style={styles.container} testID="adapt-insight-screen">
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Adapted for You</Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView style={styles.scrollView} showsVerticalScrollIndicator={false}>
        {/* Success indicator */}
        <View style={styles.successBanner}>
          <View style={styles.successIcon}>
            <Feather name="check" size={24} color={colors.text.primary} />
          </View>
          <Text style={styles.successText}>
            Pierre has personalized this insight for your training
          </Text>
        </View>

        {/* Adapted content card */}
        <View style={styles.contentCard}>
          <View style={styles.cardHeader}>
            <Feather name="refresh-cw" size={20} color={colors.pierre.violet} />
            <Text style={styles.cardTitle}>Your Personalized Version</Text>
          </View>
          <Text style={styles.adaptedContent}>{adaptedInsight.adapted_content}</Text>
          {adaptedInsight.adaptation_context && (
            <View style={styles.contextSection}>
              <Text style={styles.contextLabel}>How this was adapted:</Text>
              <Text style={styles.contextText}>{adaptedInsight.adaptation_context}</Text>
            </View>
          )}
        </View>

        {/* Meta info */}
        <View style={styles.metaCard}>
          <View style={styles.metaRow}>
            <Feather name="calendar" size={16} color={colors.text.tertiary} />
            <Text style={styles.metaText}>
              Created {formatDate(adaptedInsight.created_at)}
            </Text>
          </View>
        </View>

        {/* How it works section */}
        <View style={styles.infoSection}>
          <Text style={styles.infoTitle}>How "Adapt to My Training" Works</Text>
          <View style={styles.infoItem}>
            <View style={styles.infoBullet}>
              <Text style={styles.infoBulletText}>1</Text>
            </View>
            <Text style={styles.infoText}>
              Your friend shared a coach insight from their training
            </Text>
          </View>
          <View style={styles.infoItem}>
            <View style={styles.infoBullet}>
              <Text style={styles.infoBulletText}>2</Text>
            </View>
            <Text style={styles.infoText}>
              Pierre analyzed your recent activities and fitness profile
            </Text>
          </View>
          <View style={styles.infoItem}>
            <View style={styles.infoBullet}>
              <Text style={styles.infoBulletText}>3</Text>
            </View>
            <Text style={styles.infoText}>
              The insight was personalized to match your current training phase
            </Text>
          </View>
        </View>

        {/* Actions */}
        <View style={styles.actionsSection}>
          <TouchableOpacity
            style={styles.primaryAction}
            onPress={() => navigation.navigate('SocialFeed')}
          >
            <Feather name="home" size={18} color={colors.text.primary} />
            <Text style={styles.primaryActionText}>Back to Feed</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={styles.secondaryAction}
            onPress={() => navigation.navigate('AdaptedInsights')}
          >
            <Feather name="list" size={18} color={colors.pierre.violet} />
            <Text style={styles.secondaryActionText}>View All Adapted Insights</Text>
          </TouchableOpacity>
        </View>

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </SafeAreaView>
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
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  backButton: {
    padding: spacing.sm,
  },
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '700',
    color: colors.text.primary,
    textAlign: 'center',
  },
  headerSpacer: {
    width: 40,
  },
  scrollView: {
    flex: 1,
    paddingHorizontal: spacing.md,
  },
  successBanner: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.pierre.activity + '20',
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    marginTop: spacing.md,
    gap: spacing.md,
  },
  successIcon: {
    width: 40,
    height: 40,
    borderRadius: 20,
    backgroundColor: colors.pierre.activity,
    justifyContent: 'center',
    alignItems: 'center',
  },
  successText: {
    flex: 1,
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  contentCard: {
    marginTop: spacing.lg,
    borderRadius: borderRadius.lg,
    padding: spacing.lg,
    ...glassCard,
  },
  cardHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: spacing.md,
    gap: spacing.sm,
  },
  cardTitle: {
    color: colors.pierre.violet,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  adaptedContent: {
    color: colors.text.primary,
    fontSize: fontSize.lg,
    lineHeight: 28,
  },
  contextSection: {
    marginTop: spacing.lg,
    paddingTop: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  contextLabel: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
    fontWeight: '500',
    marginBottom: spacing.sm,
  },
  contextText: {
    color: colors.text.secondary,
    fontSize: fontSize.md,
    lineHeight: 22,
  },
  metaCard: {
    marginTop: spacing.md,
    padding: spacing.md,
    borderRadius: borderRadius.md,
    backgroundColor: colors.background.secondary,
  },
  metaRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
  },
  metaText: {
    color: colors.text.tertiary,
    fontSize: fontSize.sm,
  },
  infoSection: {
    marginTop: spacing.xl,
    padding: spacing.lg,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.background.secondary,
  },
  infoTitle: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '700',
    marginBottom: spacing.lg,
  },
  infoItem: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    marginBottom: spacing.md,
    gap: spacing.md,
  },
  infoBullet: {
    width: 24,
    height: 24,
    borderRadius: 12,
    backgroundColor: colors.pierre.violet + '30',
    justifyContent: 'center',
    alignItems: 'center',
  },
  infoBulletText: {
    color: colors.pierre.violet,
    fontSize: fontSize.sm,
    fontWeight: '700',
  },
  infoText: {
    flex: 1,
    color: colors.text.secondary,
    fontSize: fontSize.md,
    lineHeight: 22,
  },
  actionsSection: {
    marginTop: spacing.xl,
    gap: spacing.md,
  },
  primaryAction: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.pierre.violet,
    gap: spacing.sm,
  },
  primaryActionText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  secondaryAction: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: spacing.md,
    borderRadius: borderRadius.lg,
    backgroundColor: colors.background.secondary,
    gap: spacing.sm,
  },
  secondaryActionText: {
    color: colors.pierre.violet,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
  bottomSpacer: {
    height: spacing.xl,
  },
});
