// ABOUTME: Root navigation component handling auth state
// ABOUTME: Shows AuthStack for unauthenticated users, AppNavigator for authenticated users

import React from 'react';
import { View, ActivityIndicator } from 'react-native';
import { NavigationContainer } from '@react-navigation/native';
import { useAuth } from '../contexts/AuthContext';
import { AuthStack } from './AuthStack';
import { AppNavigator } from './AppNavigator';
import { colors } from '../constants/theme';

export function RootNavigator() {
  const { isAuthenticated, isLoading, user } = useAuth();

  // Show loading screen while checking auth state
  if (isLoading) {
    return (
      <View className="flex-1 items-center justify-center bg-background-primary">
        <ActivityIndicator size="large" color={colors.primary[500]} />
      </View>
    );
  }

  // If user is pending approval, show auth stack (will navigate to pending screen)
  const showAuthStack = !isAuthenticated || user?.user_status === 'pending';

  return (
    <NavigationContainer>
      {showAuthStack ? <AuthStack /> : <AppNavigator />}
    </NavigationContainer>
  );
}
