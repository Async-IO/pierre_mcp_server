// ABOUTME: Pending approval screen shown after registration
// ABOUTME: Professional dark theme UI with glassmorphism matching web design

import React from 'react';
import {
  View,
  Text,
  SafeAreaView,
  TouchableOpacity,
  Image,
  ScrollView,
  type ImageStyle,
  type ViewStyle,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { Button } from '../../components/ui';
import { spacing, glassCard, gradients } from '../../constants/theme';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';

type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
  PendingApproval: undefined;
};

interface PendingApprovalScreenProps {
  navigation: NativeStackNavigationProp<AuthStackParamList, 'PendingApproval'>;
}

// Logo style (pixel-specific dimensions)
const logoStyle: ImageStyle = { width: 100, height: 100, marginBottom: spacing.md };

// Glassmorphism card style
const cardStyle: ViewStyle = {
  ...glassCard,
  borderRadius: 16,
  overflow: 'hidden',
};

// Step badge style
const stepBadgeStyle: ViewStyle = {
  width: 24,
  height: 24,
  borderRadius: 12,
  alignItems: 'center',
  justifyContent: 'center',
  marginRight: spacing.sm,
};

export function PendingApprovalScreen({ navigation }: PendingApprovalScreenProps) {
  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      <ScrollView
        contentContainerStyle={{ flexGrow: 1, justifyContent: 'center', paddingHorizontal: spacing.lg, paddingVertical: spacing.xl }}
      >
        {/* Glassmorphism Card Container */}
        <View style={cardStyle}>
          {/* Gradient accent bar at top */}
          <LinearGradient
            colors={gradients.violetCyan as [string, string]}
            start={{ x: 0, y: 0 }}
            end={{ x: 1, y: 0 }}
            style={{ height: 3, width: '100%' }}
          />

          <View className="px-6 py-8">
            {/* Pierre Logo */}
            <View className="items-center">
              <Image
                source={require('../../../assets/pierre-logo.png')}
                style={logoStyle}
                resizeMode="contain"
              />
            </View>

            {/* Message */}
            <Text className="text-xl font-bold text-text-primary text-center mb-3">
              Account Pending Approval
            </Text>
            <Text className="text-sm text-text-secondary text-center leading-5 mb-2">
              Thank you for registering with Pierre! Your account is currently
              being reviewed by our team.
            </Text>
            <Text className="text-sm text-text-secondary text-center leading-5 mb-4">
              You'll receive an email notification once your account has been
              approved and is ready to use.
            </Text>

            {/* Info Box with glassmorphism */}
            <View className="bg-background-tertiary rounded-xl p-4 mb-6 border border-border-subtle">
              <Text className="text-base font-semibold text-text-primary mb-3">
                What happens next?
              </Text>
              <View className="flex-row items-center mb-3">
                <LinearGradient
                  colors={gradients.violetCyan as [string, string]}
                  style={stepBadgeStyle}
                >
                  <Text className="text-white text-xs font-bold">1</Text>
                </LinearGradient>
                <Text className="flex-1 text-sm text-text-secondary">
                  Our team reviews your registration
                </Text>
              </View>
              <View className="flex-row items-center mb-3">
                <LinearGradient
                  colors={gradients.violetCyan as [string, string]}
                  style={stepBadgeStyle}
                >
                  <Text className="text-white text-xs font-bold">2</Text>
                </LinearGradient>
                <Text className="flex-1 text-sm text-text-secondary">
                  You'll receive an approval email
                </Text>
              </View>
              <View className="flex-row items-center">
                <LinearGradient
                  colors={gradients.violetCyan as [string, string]}
                  style={stepBadgeStyle}
                >
                  <Text className="text-white text-xs font-bold">3</Text>
                </LinearGradient>
                <Text className="flex-1 text-sm text-text-secondary">
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
              style={{ marginBottom: spacing.md }}
            />

            {/* Contact Support */}
            <TouchableOpacity className="items-center">
              <Text className="text-xs text-text-tertiary">
                Questions? Contact support@pierre.fitness
              </Text>
            </TouchableOpacity>
          </View>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}
