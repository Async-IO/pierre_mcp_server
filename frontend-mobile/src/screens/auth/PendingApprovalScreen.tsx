// ABOUTME: Pending approval screen shown after registration
// ABOUTME: Informs user their account is awaiting admin approval

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  TouchableOpacity,
  Image,
} from 'react-native';
import { Button } from '../../components/ui';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';

type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
  PendingApproval: undefined;
};

interface PendingApprovalScreenProps {
  navigation: NativeStackNavigationProp<AuthStackParamList, 'PendingApproval'>;
}

export function PendingApprovalScreen({ navigation }: PendingApprovalScreenProps) {
  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.content}>
        {/* Pierre Logo */}
        <Image
          source={require('../../../assets/pierre-logo.png')}
          style={styles.logo}
          resizeMode="contain"
        />

        {/* Message */}
        <Text style={styles.title}>Account Pending Approval</Text>
        <Text style={styles.description}>
          Thank you for registering with Pierre! Your account is currently
          being reviewed by our team.
        </Text>
        <Text style={styles.description}>
          You'll receive an email notification once your account has been
          approved and is ready to use.
        </Text>

        {/* Info Box */}
        <View style={styles.infoBox}>
          <Text style={styles.infoTitle}>What happens next?</Text>
          <View style={styles.infoItem}>
            <Text style={styles.bullet}>1</Text>
            <Text style={styles.infoText}>
              Our team reviews your registration
            </Text>
          </View>
          <View style={styles.infoItem}>
            <Text style={styles.bullet}>2</Text>
            <Text style={styles.infoText}>
              You'll receive an approval email
            </Text>
          </View>
          <View style={styles.infoItem}>
            <Text style={styles.bullet}>3</Text>
            <Text style={styles.infoText}>
              Sign in and connect your fitness accounts
            </Text>
          </View>
        </View>

        {/* Back to Login */}
        <Button
          title="Back to Sign In"
          onPress={() => navigation.navigate('Login')}
          variant="secondary"
          fullWidth
          style={styles.backButton}
        />

        {/* Contact Support */}
        <TouchableOpacity style={styles.supportLink}>
          <Text style={styles.supportText}>
            Questions? Contact support@pierre.fitness
          </Text>
        </TouchableOpacity>
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  content: {
    flex: 1,
    justifyContent: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.xl,
  },
  logo: {
    width: 120,
    height: 120,
    alignSelf: 'center',
    marginBottom: spacing.lg,
  },
  title: {
    fontSize: fontSize.xxl,
    fontWeight: '700',
    color: colors.text.primary,
    textAlign: 'center',
    marginBottom: spacing.md,
  },
  description: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
    lineHeight: 24,
    marginBottom: spacing.sm,
  },
  infoBox: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    marginTop: spacing.xl,
    marginBottom: spacing.xl,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  infoTitle: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.md,
  },
  infoItem: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: spacing.sm,
  },
  bullet: {
    width: 24,
    height: 24,
    borderRadius: 12,
    backgroundColor: colors.primary[600],
    color: colors.text.primary,
    fontSize: fontSize.sm,
    fontWeight: '600',
    textAlign: 'center',
    lineHeight: 24,
    marginRight: spacing.sm,
    overflow: 'hidden',
  },
  infoText: {
    flex: 1,
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  backButton: {
    marginBottom: spacing.md,
  },
  supportLink: {
    alignItems: 'center',
  },
  supportText: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
  },
});
