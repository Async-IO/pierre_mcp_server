// ABOUTME: Bottom tab navigation for primary app screens (Home, Chat, Coaches, Activity, Profile)
// ABOUTME: Each tab has its own stack navigator so detail screens keep the tab bar visible

import React from 'react';
import { View, TouchableOpacity, Text, type ViewStyle } from 'react-native';
import {
  createBottomTabNavigator,
  type BottomTabBarProps,
} from '@react-navigation/bottom-tabs';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { Feather } from '@expo/vector-icons';
import { colors, spacing } from '../constants/theme';
import { HomeScreen } from '../screens/HomeScreen';
import { ChatScreen } from '../screens/chat/ChatScreen';
import { ConversationsScreen } from '../screens/conversations/ConversationsScreen';
import { SocialFeedScreen } from '../screens/social/SocialFeedScreen';
import { FriendsScreen } from '../screens/social/FriendsScreen';
import { SearchFriendsScreen } from '../screens/social/SearchFriendsScreen';
import { FriendRequestsScreen } from '../screens/social/FriendRequestsScreen';
import { AdaptedInsightsScreen } from '../screens/social/AdaptedInsightsScreen';
import { CoachLibraryScreen } from '../screens/coaches/CoachLibraryScreen';
import { CoachDetailScreen } from '../screens/coaches/CoachDetailScreen';
import { CoachEditorScreen } from '../screens/coaches/CoachEditorScreen';
import { CoachWizardScreen } from '../screens/coaches/CoachWizardScreen';
import { StoreScreen } from '../screens/store/StoreScreen';
import { StoreCoachDetailScreen } from '../screens/store/StoreCoachDetailScreen';
import { SettingsScreen } from '../screens/settings/SettingsScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { SocialSettingsScreen } from '../screens/social/SocialSettingsScreen';
import { ShareInsightScreen } from '../screens/social/ShareInsightScreen';
import { AdaptedInsightScreen } from '../screens/social/AdaptedInsightScreen';
import type { AdaptedInsight } from '../types';

// Stack param lists for each tab
export type HomeStackParamList = {
  HomeMain: undefined;
};

export type ChatStackParamList = {
  ChatMain: { conversationId?: string } | undefined;
  Conversations: undefined;
};

export type SocialStackParamList = {
  SocialMain: undefined;
  Friends: undefined;
  SearchFriends: undefined;
  FriendRequests: undefined;
  AdaptedInsights: undefined;
  AdaptedInsight: { adaptedInsight: AdaptedInsight };
  ShareInsight: { activityId?: string } | undefined;
  SocialSettings: undefined;
};

export type CoachesStackParamList = {
  CoachesMain: undefined;
  CoachDetail: { coachId: string };
  CoachEditor: { coachId?: string } | undefined;
  CoachWizard: { coachId?: string } | undefined;
  Store: undefined;
  StoreCoachDetail: { coachId: string };
};

export type SettingsStackParamList = {
  SettingsMain: undefined;
  Connections: undefined;
};

// Tab param list references the stacks
export type MainTabsParamList = {
  HomeTab: undefined;
  ChatTab: undefined;
  CoachesTab: undefined;
  SocialTab: undefined;
  SettingsTab: undefined;
};

const Tab = createBottomTabNavigator<MainTabsParamList>();
const HomeStack = createNativeStackNavigator<HomeStackParamList>();
const ChatStack = createNativeStackNavigator<ChatStackParamList>();
const SocialStack = createNativeStackNavigator<SocialStackParamList>();
const CoachesStack = createNativeStackNavigator<CoachesStackParamList>();
const SettingsStack = createNativeStackNavigator<SettingsStackParamList>();

// Stack navigators for each tab
function HomeStackScreen() {
  return (
    <HomeStack.Navigator screenOptions={{ headerShown: false }}>
      <HomeStack.Screen name="HomeMain" component={HomeScreen} />
    </HomeStack.Navigator>
  );
}

function ChatStackScreen() {
  return (
    <ChatStack.Navigator screenOptions={{ headerShown: false }}>
      <ChatStack.Screen name="ChatMain" component={ChatScreen} />
      <ChatStack.Screen name="Conversations" component={ConversationsScreen} />
    </ChatStack.Navigator>
  );
}

function SocialStackScreen() {
  return (
    <SocialStack.Navigator screenOptions={{ headerShown: false }}>
      <SocialStack.Screen name="SocialMain" component={SocialFeedScreen} />
      <SocialStack.Screen name="Friends" component={FriendsScreen} />
      <SocialStack.Screen name="SearchFriends" component={SearchFriendsScreen} />
      <SocialStack.Screen name="FriendRequests" component={FriendRequestsScreen} />
      <SocialStack.Screen name="AdaptedInsights" component={AdaptedInsightsScreen} />
      <SocialStack.Screen name="AdaptedInsight" component={AdaptedInsightScreen} />
      <SocialStack.Screen name="ShareInsight" component={ShareInsightScreen} />
      <SocialStack.Screen name="SocialSettings" component={SocialSettingsScreen} />
    </SocialStack.Navigator>
  );
}

function CoachesStackScreen() {
  return (
    <CoachesStack.Navigator screenOptions={{ headerShown: false }}>
      <CoachesStack.Screen name="CoachesMain" component={CoachLibraryScreen} />
      <CoachesStack.Screen name="CoachDetail" component={CoachDetailScreen} />
      <CoachesStack.Screen name="CoachEditor" component={CoachEditorScreen} />
      <CoachesStack.Screen name="CoachWizard" component={CoachWizardScreen} />
      <CoachesStack.Screen name="Store" component={StoreScreen} />
      <CoachesStack.Screen name="StoreCoachDetail" component={StoreCoachDetailScreen} />
    </CoachesStack.Navigator>
  );
}

function SettingsStackScreen() {
  return (
    <SettingsStack.Navigator screenOptions={{ headerShown: false }}>
      <SettingsStack.Screen name="SettingsMain" component={SettingsScreen} />
      <SettingsStack.Screen name="Connections" component={ConnectionsScreen} />
    </SettingsStack.Navigator>
  );
}

interface TabBarIconProps {
  focused: boolean;
  name: keyof typeof Feather.glyphMap;
  label: string;
}

function TabBarIcon({ focused, name, label }: TabBarIconProps) {
  return (
    <View className="items-center justify-center">
      <Feather
        name={name}
        size={22}
        color={focused ? colors.pierre.violet : colors.text.tertiary}
      />
      <Text
        className={`text-[10px] font-medium mt-0.5`}
        style={{ color: focused ? colors.pierre.violet : colors.text.tertiary }}
      >
        {label}
      </Text>
    </View>
  );
}

// Active indicator style (pixel-specific positioning)
const activeIndicatorStyle: ViewStyle = {
  position: 'absolute',
  bottom: -spacing.xs,
  width: 4,
  height: 4,
  borderRadius: 2,
  backgroundColor: colors.pierre.violet,
};

function CustomTabBar({ state, navigation }: BottomTabBarProps) {
  const insets = useSafeAreaInsets();

  const tabConfig: Record<string, { icon: keyof typeof Feather.glyphMap; label: string }> = {
    HomeTab: { icon: 'home', label: 'Home' },
    ChatTab: { icon: 'message-circle', label: 'Chat' },
    CoachesTab: { icon: 'award', label: 'Coaches' },
    SocialTab: { icon: 'activity', label: 'Activity' },
    SettingsTab: { icon: 'user', label: 'Profile' },
  };

  return (
    <View
      className="flex-row bg-background-secondary border-t border-border-subtle pt-2"
      style={{ paddingBottom: insets.bottom || spacing.sm }}
    >
      {state.routes.map((route, tabIndex) => {
        const isFocused = state.index === tabIndex;
        const config = tabConfig[route.name] || { icon: 'circle', label: route.name };

        const onPress = () => {
          const event = navigation.emit({
            type: 'tabPress',
            target: route.key,
            canPreventDefault: true,
          });

          if (!isFocused && !event.defaultPrevented) {
            navigation.navigate(route.name);
          }
        };

        const onLongPress = () => {
          navigation.emit({
            type: 'tabLongPress',
            target: route.key,
          });
        };

        return (
          <TouchableOpacity
            key={route.key}
            accessibilityRole="button"
            accessibilityState={isFocused ? { selected: true } : {}}
            accessibilityLabel={config.label}
            testID={`tab-${route.name.toLowerCase()}`}
            onPress={onPress}
            onLongPress={onLongPress}
            className="flex-1 items-center justify-center py-1"
          >
            <TabBarIcon
              focused={isFocused}
              name={config.icon}
              label={config.label}
            />
            {isFocused && <View style={activeIndicatorStyle} />}
          </TouchableOpacity>
        );
      })}
    </View>
  );
}

export function MainTabs() {
  return (
    <Tab.Navigator
      tabBar={(props: BottomTabBarProps) => <CustomTabBar {...props} />}
      screenOptions={{
        headerShown: false,
      }}
    >
      <Tab.Screen
        name="HomeTab"
        component={HomeStackScreen}
        options={{ tabBarLabel: 'Home' }}
      />
      <Tab.Screen
        name="ChatTab"
        component={ChatStackScreen}
        options={{ tabBarLabel: 'Chat' }}
      />
      <Tab.Screen
        name="CoachesTab"
        component={CoachesStackScreen}
        options={{ tabBarLabel: 'Coaches' }}
      />
      <Tab.Screen
        name="SocialTab"
        component={SocialStackScreen}
        options={{ tabBarLabel: 'Activity' }}
      />
      <Tab.Screen
        name="SettingsTab"
        component={SettingsStackScreen}
        options={{ tabBarLabel: 'Profile' }}
      />
    </Tab.Navigator>
  );
}

