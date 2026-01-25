// ABOUTME: Pending approval screen shown after registration
// ABOUTME: Informs user their account is awaiting admin approval

import React from 'react';
import {
  View,
  Text,
  SafeAreaView,
  TouchableOpacity,
  Image,
  type ImageStyle,
} from 'react-native';
import { Button } from '../../components/ui';
import { spacing } from '../../constants/theme';
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
const logoStyle: ImageStyle = { width: 120, height: 120, alignSelf: 'center', marginBottom: spacing.lg };

export function PendingApprovalScreen({ navigation }: PendingApprovalScreenProps) {
  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      <View className="flex-1 justify-center px-5 py-6">
        {/* Pierre Logo */}
        <Image
          source={require('../../../assets/pierre-logo.png')}
          style={logoStyle}
          resizeMode="contain"
        />

        {/* Message */}
        <Text className="text-2xl font-bold text-text-primary text-center mb-3">
          Account Pending Approval
        </Text>
        <Text className="text-base text-text-secondary text-center leading-6 mb-2">
          Thank you for registering with Pierre! Your account is currently
          being reviewed by our team.
        </Text>
        <Text className="text-base text-text-secondary text-center leading-6 mb-2">
          You'll receive an email notification once your account has been
          approved and is ready to use.
        </Text>

        {/* Info Box */}
        <View className="bg-background-secondary rounded-lg p-3 mt-6 mb-6 border border-border-subtle">
          <Text className="text-base font-semibold text-text-primary mb-3">
            What happens next?
          </Text>
          <View className="flex-row items-center mb-2">
            <Text className="w-6 h-6 rounded-full bg-primary-600 text-text-primary text-sm font-semibold text-center leading-6 mr-2 overflow-hidden">
              1
            </Text>
            <Text className="flex-1 text-sm text-text-secondary">
              Our team reviews your registration
            </Text>
          </View>
          <View className="flex-row items-center mb-2">
            <Text className="w-6 h-6 rounded-full bg-primary-600 text-text-primary text-sm font-semibold text-center leading-6 mr-2 overflow-hidden">
              2
            </Text>
            <Text className="flex-1 text-sm text-text-secondary">
              You'll receive an approval email
            </Text>
          </View>
          <View className="flex-row items-center mb-2">
            <Text className="w-6 h-6 rounded-full bg-primary-600 text-text-primary text-sm font-semibold text-center leading-6 mr-2 overflow-hidden">
              3
            </Text>
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
          <Text className="text-sm text-text-tertiary">
            Questions? Contact support@pierre.fitness
          </Text>
        </TouchableOpacity>
      </View>
    </SafeAreaView>
  );
}
