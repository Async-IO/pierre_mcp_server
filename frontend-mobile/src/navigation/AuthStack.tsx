// ABOUTME: Authentication navigation stack
// ABOUTME: Contains Login, Register, and PendingApproval screens

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { LoginScreen } from '../screens/auth/LoginScreen';
import { RegisterScreen } from '../screens/auth/RegisterScreen';
import { PendingApprovalScreen } from '../screens/auth/PendingApprovalScreen';
import { useAuth } from '../contexts/AuthContext';
import { colors } from '../constants/theme';

export type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
  PendingApproval: undefined;
};

const Stack = createNativeStackNavigator<AuthStackParamList>();

export function AuthStack() {
  const { user } = useAuth();

  // If user is logged in but pending, show PendingApproval as initial screen
  const initialRouteName = user?.user_status === 'pending' ? 'PendingApproval' : 'Login';

  return (
    <Stack.Navigator
      initialRouteName={initialRouteName}
      screenOptions={{
        headerShown: false,
        contentStyle: { backgroundColor: colors.background.primary },
        animation: 'slide_from_right',
      }}
    >
      <Stack.Screen name="Login" component={LoginScreen} />
      <Stack.Screen name="Register" component={RegisterScreen} />
      <Stack.Screen name="PendingApproval" component={PendingApprovalScreen} />
    </Stack.Navigator>
  );
}
