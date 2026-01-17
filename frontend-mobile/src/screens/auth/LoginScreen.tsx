// ABOUTME: Login screen with email/password and Google Sign-In authentication
// ABOUTME: Professional dark theme UI matching ChatGPT/Claude design aesthetic

import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  TouchableOpacity,
  Alert,
  Image,
  ActivityIndicator,
} from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { Button, Input } from '../../components/ui';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
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

  return (
    <SafeAreaView style={styles.container} testID="login-screen">
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        <ScrollView
          contentContainerStyle={styles.scrollContent}
          keyboardShouldPersistTaps="handled"
        >
          {/* Logo and Header */}
          <View style={styles.header}>
            <Image
              source={require('../../../assets/pierre-logo.png')}
              style={styles.logo}
              resizeMode="contain"
            />
            <Text style={styles.title}>Welcome to Pierre</Text>
            <Text style={styles.subtitle}>
              Your AI-powered fitness intelligence companion
            </Text>
          </View>

          {/* Login Form */}
          <View style={styles.form}>
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
              style={styles.loginButton}
              testID="login-button"
            />

            {/* Google Sign-In - only show when Firebase is configured */}
            {isFirebaseEnabled() && (
              <>
                <View style={styles.dividerContainer}>
                  <View style={styles.dividerLine} />
                  <Text style={styles.dividerText}>or continue with</Text>
                  <View style={styles.dividerLine} />
                </View>

                <TouchableOpacity
                  style={styles.googleButton}
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
                  <Text style={styles.googleButtonText}>
                    {isGoogleLoading ? 'Signing in...' : 'Continue with Google'}
                  </Text>
                </TouchableOpacity>
              </>
            )}
          </View>

          {/* Register Link */}
          <View style={styles.footer}>
            <Text style={styles.footerText}>Don't have an account?</Text>
            <TouchableOpacity onPress={() => navigation.navigate('Register')}>
              <Text style={styles.linkText}>Create one</Text>
            </TouchableOpacity>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  keyboardView: {
    flex: 1,
  },
  scrollContent: {
    flexGrow: 1,
    justifyContent: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.xl,
  },
  header: {
    alignItems: 'center',
    marginBottom: spacing.xl,
  },
  logo: {
    width: 160,
    height: 160,
    marginBottom: spacing.lg,
  },
  title: {
    fontSize: fontSize.xxl,
    fontWeight: '700',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  subtitle: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
    lineHeight: 22,
  },
  form: {
    marginBottom: spacing.xl,
  },
  loginButton: {
    marginTop: spacing.md,
  },
  footer: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
    gap: spacing.xs,
  },
  footerText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
  },
  linkText: {
    color: colors.primary[500],
    fontSize: fontSize.sm,
    fontWeight: '600',
  },
  dividerContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginVertical: spacing.lg,
  },
  dividerLine: {
    flex: 1,
    height: 1,
    backgroundColor: colors.border.default,
  },
  dividerText: {
    color: colors.text.secondary,
    fontSize: fontSize.sm,
    paddingHorizontal: spacing.md,
  },
  googleButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.background.secondary,
    borderWidth: 1,
    borderColor: colors.border.default,
    borderRadius: borderRadius.md,
    paddingVertical: spacing.md,
    paddingHorizontal: spacing.lg,
    gap: spacing.sm,
  },
  googleButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '500',
  },
});
