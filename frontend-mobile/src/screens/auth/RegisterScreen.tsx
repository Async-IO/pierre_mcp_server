// ABOUTME: Registration screen for new user signup
// ABOUTME: Professional dark theme UI with glassmorphism matching web design

import React, { useState } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  TouchableOpacity,
  Alert,
  type ViewStyle,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { useAuth } from '../../contexts/AuthContext';
import { Button, Input } from '../../components/ui';
import { spacing, glassCard, buttonGlow, gradients } from '../../constants/theme';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';

type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
  PendingApproval: undefined;
};

interface RegisterScreenProps {
  navigation: NativeStackNavigationProp<AuthStackParamList, 'Register'>;
}

export function RegisterScreen({ navigation }: RegisterScreenProps) {
  const { register } = useAuth();
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [errors, setErrors] = useState<{
    displayName?: string;
    email?: string;
    password?: string;
    confirmPassword?: string;
  }>({});

  const validateForm = () => {
    const newErrors: typeof errors = {};

    if (!displayName.trim()) {
      newErrors.displayName = 'Display name is required';
    }

    if (!email.trim()) {
      newErrors.email = 'Email is required';
    } else if (!/\S+@\S+\.\S+/.test(email)) {
      newErrors.email = 'Please enter a valid email';
    }

    if (!password) {
      newErrors.password = 'Password is required';
    } else if (password.length < 8) {
      newErrors.password = 'Password must be at least 8 characters';
    }

    if (!confirmPassword) {
      newErrors.confirmPassword = 'Please confirm your password';
    } else if (password !== confirmPassword) {
      newErrors.confirmPassword = 'Passwords do not match';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleRegister = async () => {
    if (!validateForm()) return;

    setIsLoading(true);
    try {
      await register(email.trim(), password, displayName.trim());
      // Navigate to pending approval screen
      navigation.replace('PendingApproval');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Registration failed';
      Alert.alert('Registration Failed', message);
    } finally {
      setIsLoading(false);
    }
  };

  // Glassmorphism card style
  const cardStyle: ViewStyle = {
    ...glassCard,
    borderRadius: 16,
    overflow: 'hidden',
  };

  // Button with glow effect
  const glowButtonStyle: ViewStyle = {
    ...buttonGlow,
    marginTop: spacing.md,
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        <ScrollView
          contentContainerStyle={{ flexGrow: 1, justifyContent: 'center', paddingHorizontal: spacing.lg, paddingVertical: spacing.xl }}
          keyboardShouldPersistTaps="handled"
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
              {/* Header */}
              <View className="items-center mb-6">
                <LinearGradient
                  colors={gradients.violetCyan as [string, string]}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 1 }}
                  className="w-14 h-14 rounded-xl items-center justify-center mb-3"
                >
                  <Text className="text-[28px] font-bold text-white">P</Text>
                </LinearGradient>
                <Text className="text-xl font-bold text-text-primary mb-1">Create Account</Text>
                <Text className="text-sm text-text-secondary text-center leading-[20px]">
                  Join Pierre to unlock AI-powered fitness insights
                </Text>
              </View>

              {/* Registration Form */}
              <View className="mb-4">
                <Input
                  label="Display Name"
                  placeholder="How should we call you?"
                  value={displayName}
                  onChangeText={setDisplayName}
                  autoCapitalize="words"
                  error={errors.displayName}
                />

                <Input
                  label="Email"
                  placeholder="you@example.com"
                  value={email}
                  onChangeText={setEmail}
                  keyboardType="email-address"
                  autoCapitalize="none"
                  autoCorrect={false}
                  error={errors.email}
                />

                <Input
                  label="Password"
                  placeholder="Minimum 8 characters"
                  value={password}
                  onChangeText={setPassword}
                  secureTextEntry
                  showPasswordToggle
                  error={errors.password}
                />

                <Input
                  label="Confirm Password"
                  placeholder="Re-enter your password"
                  value={confirmPassword}
                  onChangeText={setConfirmPassword}
                  secureTextEntry
                  showPasswordToggle
                  error={errors.confirmPassword}
                />

                <Button
                  title="Create Account"
                  onPress={handleRegister}
                  loading={isLoading}
                  fullWidth
                  style={glowButtonStyle}
                />
              </View>

              {/* Login Link */}
              <View className="flex-row justify-center items-center gap-1 pt-2">
                <Text className="text-sm text-text-secondary">Already have an account?</Text>
                <TouchableOpacity onPress={() => navigation.navigate('Login')}>
                  <Text className="text-sm font-semibold text-primary-500">Sign in</Text>
                </TouchableOpacity>
              </View>
            </View>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
