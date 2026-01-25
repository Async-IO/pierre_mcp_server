// ABOUTME: Login screen with email/password and Google Sign-In authentication
// ABOUTME: Professional dark theme UI matching ChatGPT/Claude design aesthetic

import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  TouchableOpacity,
  Alert,
  Image,
  ActivityIndicator,
  type ImageStyle,
} from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { Button, Input } from '../../components/ui';
import { colors, spacing } from '../../constants/theme';
import {
  isFirebaseEnabled,
  useGoogleAuth,
  signInWithGoogleResponse,
} from '../../firebase';
import { AntDesign } from '@expo/vector-icons';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';

type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
  PendingApproval: undefined;
};

interface LoginScreenProps {
  navigation: NativeStackNavigationProp<AuthStackParamList, 'Login'>;
}

export function LoginScreen({ navigation }: LoginScreenProps) {
  const { login, loginWithFirebase } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isGoogleLoading, setIsGoogleLoading] = useState(false);
  const [errors, setErrors] = useState<{ email?: string; password?: string }>({});

  // Google OAuth hook - always call unconditionally (React Rules of Hooks)
  // Hook returns null values when Firebase is not enabled
  const googleAuth = useGoogleAuth();

  // Handle Google OAuth response
  useEffect(() => {
    if (!googleAuth?.response) return;

    const handleGoogleResponse = async () => {
      if (googleAuth.response?.type === 'success') {
        setIsGoogleLoading(true);
        try {
          const result = await signInWithGoogleResponse(googleAuth.response);
          if (result) {
            await loginWithFirebase(result.idToken);
            // Navigation handled by auth state change
          }
        } catch (error) {
          let message = 'Google sign-in failed. Please try again.';
          if (error instanceof Error) {
            message = error.message;
          }
          Alert.alert('Sign In Failed', message);
        } finally {
          setIsGoogleLoading(false);
        }
      } else if (googleAuth.response?.type === 'error') {
        Alert.alert('Sign In Error', 'Google sign-in was cancelled or failed.');
        setIsGoogleLoading(false);
      } else {
        // Handle 'dismiss', 'cancel', 'locked' - reset loading state
        setIsGoogleLoading(false);
      }
    };

    handleGoogleResponse();
  }, [googleAuth.response, loginWithFirebase]);

  const validateForm = () => {
    const newErrors: { email?: string; password?: string } = {};

    if (!email.trim()) {
      newErrors.email = 'Email is required';
    } else if (!/\S+@\S+\.\S+/.test(email)) {
      newErrors.email = 'Please enter a valid email';
    }

    if (!password) {
      newErrors.password = 'Password is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleLogin = async () => {
    if (!validateForm()) return;

    setIsLoading(true);
    try {
      await login(email.trim(), password);
      // Navigation is handled by auth state change in RootNavigator/AuthStack
      // If user is pending, AuthStack will show PendingApproval screen
    } catch (error) {
      let message = 'Login failed. Please try again.';
      if (error instanceof Error) {
        // Parse API error responses
        if (error.message.includes('400') || error.message.includes('invalid')) {
          message = 'Invalid email or password. Please check your credentials.';
        } else if (error.message.includes('Network')) {
          message = 'Network error. Please check your connection.';
        } else {
          message = error.message;
        }
      }
      Alert.alert('Login Failed', message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleGoogleSignIn = async () => {
    if (!googleAuth?.promptAsync) {
      Alert.alert('Not Available', 'Google Sign-In is not configured.');
      return;
    }

    setIsGoogleLoading(true);
    try {
      await googleAuth.promptAsync();
      // Response is handled in useEffect above
    } catch (error) {
      let message = 'Failed to start Google sign-in.';
      if (error instanceof Error) {
        message = error.message;
      }
      Alert.alert('Error', message);
      setIsGoogleLoading(false);
    }
  };

  // Logo style (pixel-specific dimensions)
  const logoStyle: ImageStyle = { width: 160, height: 160, marginBottom: spacing.lg };

  return (
    <SafeAreaView className="flex-1 bg-background-primary" testID="login-screen">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        <ScrollView
          contentContainerStyle={{ flexGrow: 1, justifyContent: 'center', paddingHorizontal: spacing.lg, paddingVertical: spacing.xl }}
          keyboardShouldPersistTaps="handled"
          testID="login-scroll-view"
        >
          {/* Logo and Header */}
          <View className="items-center mb-6">
            <Image
              source={require('../../../assets/pierre-logo.png')}
              style={logoStyle}
              resizeMode="contain"
            />
            <Text className="text-2xl font-bold text-text-primary mb-1">
              Welcome to Pierre
            </Text>
            <Text className="text-base text-text-secondary text-center leading-[22px]">
              Your AI-powered fitness intelligence companion
            </Text>
          </View>

          {/* Login Form */}
          <View className="mb-6">
            <Input
              label="Email"
              placeholder="you@example.com"
              value={email}
              onChangeText={setEmail}
              keyboardType="email-address"
              autoCapitalize="none"
              autoCorrect={false}
              error={errors.email}
              testID="email-input"
            />

            <Input
              label="Password"
              placeholder="Enter your password"
              value={password}
              onChangeText={setPassword}
              secureTextEntry
              showPasswordToggle
              error={errors.password}
              testID="password-input"
            />

            <Button
              title="Sign In"
              onPress={handleLogin}
              loading={isLoading}
              fullWidth
              style={{ marginTop: spacing.md }}
              testID="login-button"
            />

            {/* Google Sign-In - only show when Firebase is configured */}
            {isFirebaseEnabled() && (
              <>
                <View className="flex-row items-center my-5">
                  <View className="flex-1 h-px bg-border-default" />
                  <Text className="text-sm text-text-secondary px-3">or continue with</Text>
                  <View className="flex-1 h-px bg-border-default" />
                </View>

                <TouchableOpacity
                  className="flex-row items-center justify-center bg-background-secondary border border-border-default rounded-lg py-3 px-5 gap-2"
                  onPress={handleGoogleSignIn}
                  disabled={isGoogleLoading}
                  testID="google-signin-button"
                  activeOpacity={0.7}
                >
                  {isGoogleLoading ? (
                    <ActivityIndicator size="small" color={colors.text.primary} />
                  ) : (
                    <AntDesign name="google" size={20} color={colors.google} />
                  )}
                  <Text className="text-base font-medium text-text-primary">
                    {isGoogleLoading ? 'Signing in...' : 'Continue with Google'}
                  </Text>
                </TouchableOpacity>
              </>
            )}
          </View>

          {/* Register Link */}
          <View className="flex-row justify-center items-center gap-1">
            <Text className="text-sm text-text-secondary">Don't have an account?</Text>
            <TouchableOpacity onPress={() => navigation.navigate('Register')}>
              <Text className="text-sm font-semibold text-primary-500">Create one</Text>
            </TouchableOpacity>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
