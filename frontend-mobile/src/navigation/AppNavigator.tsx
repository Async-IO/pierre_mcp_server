// ABOUTME: Main app navigation for authenticated users using stack navigator
// ABOUTME: Wraps MainTabs with app-level modal screens (no drawer)

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { MainTabs } from './MainTabs';
import { ShareInsightScreen } from '../screens/social/ShareInsightScreen';
import { AdaptedInsightScreen } from '../screens/social/AdaptedInsightScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import type { AdaptedInsight } from '../types';

export type AppNavigatorParamList = {
  Main: undefined;
  ShareInsight: undefined;
  AdaptedInsight: { adaptedInsight: AdaptedInsight };
  Connections: undefined;
};

const Stack = createNativeStackNavigator<AppNavigatorParamList>();

export function AppNavigator() {
  return (
    <Stack.Navigator
      screenOptions={{
        headerShown: false,
      }}
    >
      <Stack.Screen name="Main" component={MainTabs} />
      <Stack.Screen
        name="ShareInsight"
        component={ShareInsightScreen}
        options={{ presentation: 'modal' }}
      />
      <Stack.Screen
        name="AdaptedInsight"
        component={AdaptedInsightScreen}
        options={{ presentation: 'modal' }}
      />
      <Stack.Screen
        name="Connections"
        component={ConnectionsScreen}
        options={{ presentation: 'modal' }}
      />
    </Stack.Navigator>
  );
}
