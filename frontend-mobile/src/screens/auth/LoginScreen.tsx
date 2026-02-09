// ABOUTME: Login screen with email/password and Google Sign-In authentication
// ABOUTME: Professional dark theme UI with glassmorphism matching web design

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
  type ViewStyle,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { useAuth } from '../../contexts/AuthContext';
import { Button, Input } from '../../components/ui';
import { colors, spacing, glassCard, buttonGlow, gradients } from '../../constants/theme';
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
  const logoStyle: ImageStyle = { width: 120, height: 120, marginBottom: spacing.md };

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
              {/* Logo and Header */}
              <View className="items-center mb-6">
                <Image
                  source={require('../../../assets/pierre-logo.png')}
                  style={logoStyle}
                  resizeMode="contain"
                />
                <Text className="text-xl font-bold text-text-primary mb-1">
                  Welcome to Pierre
                </Text>
                <Text className="text-sm text-text-secondary text-center leading-[20px]">
                  Your AI-powered fitness intelligence companion
                </Text>
              </View>

              {/* Login Form */}
              <View className="mb-4">
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
                  returnKeyType="go"
                  onSubmitEditing={handleLogin}
                  error={errors.password}
                  testID="password-input"
                />

                <Button
                  title="Sign In"
                  onPress={handleLogin}
                  loading={isLoading}
                  fullWidth
                  style={glowButtonStyle}
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
                      className="flex-row items-center justify-center bg-background-tertiary border border-border-default rounded-xl py-3 px-5 gap-2"
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
              <View className="flex-row justify-center items-center gap-1 pt-2">
                <Text className="text-sm text-text-secondary">Don't have an account?</Text>
                <TouchableOpacity onPress={() => navigation.navigate('Register')}>
                  <Text className="text-sm font-semibold text-primary-500">Create one</Text>
                </TouchableOpacity>
              </View>
            </View>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
