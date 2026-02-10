// ABOUTME: Bottom tab navigation for primary app screens (Chat, Coaches, Discover, Insights, Settings)
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
import { TabSwipeWrapper } from '../components/ui';
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
import { StoreScreen } from '../screens/store/StoreScreen';
import { StoreCoachDetailScreen } from '../screens/store/StoreCoachDetailScreen';
import { SettingsScreen } from '../screens/settings/SettingsScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { SocialSettingsScreen } from '../screens/social/SocialSettingsScreen';
import { ShareInsightScreen } from '../screens/social/ShareInsightScreen';
import { AdaptedInsightScreen } from '../screens/social/AdaptedInsightScreen';
import { ActivityDetailScreen } from '../screens/ActivityDetailScreen';
import type { AdaptedInsight } from '../types';

// Stack param lists for each tab
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
  ShareInsight: {
    activityId?: string;
    content?: string;
    insightType?: string;
    visibility?: 'friends_only' | 'public';
  } | undefined;
  SocialSettings: undefined;
  ActivityDetail: {
    activityId: string;
    activityTitle?: string;
    activityType?: string;
    activityDate?: string;
    insightContent?: string;
  };
};

export type CoachesStackParamList = {
  CoachesMain: undefined;
  CoachDetail: { coachId: string };
  CoachEditor: { coachId?: string } | undefined;
};

export type SettingsStackParamList = {
  SettingsMain: undefined;
  Connections: undefined;
};

export type DiscoverStackParamList = {
  Store: undefined;
  StoreCoachDetail: { coachId: string };
};

// Tab param list references the stacks
export type MainTabsParamList = {
  ChatTab: undefined;
  CoachesTab: undefined;
  DiscoverTab: undefined;
  SocialTab: undefined;
  SettingsTab: undefined;
};

const Tab = createBottomTabNavigator<MainTabsParamList>();
const ChatStack = createNativeStackNavigator<ChatStackParamList>();
const SocialStack = createNativeStackNavigator<SocialStackParamList>();
const CoachesStack = createNativeStackNavigator<CoachesStackParamList>();
const DiscoverStack = createNativeStackNavigator<DiscoverStackParamList>();
const SettingsStack = createNativeStackNavigator<SettingsStackParamList>();

// Stack navigators for each tab
function ChatStackScreen() {
  return (
    <TabSwipeWrapper tabName="ChatTab">
      <ChatStack.Navigator screenOptions={{ headerShown: false }}>
        <ChatStack.Screen name="ChatMain" component={ChatScreen} />
        <ChatStack.Screen name="Conversations" component={ConversationsScreen} />
      </ChatStack.Navigator>
    </TabSwipeWrapper>
  );
}

function SocialStackScreen() {
  return (
    <TabSwipeWrapper tabName="SocialTab">
      <SocialStack.Navigator screenOptions={{ headerShown: false }}>
        <SocialStack.Screen name="SocialMain" component={SocialFeedScreen} />
        <SocialStack.Screen name="Friends" component={FriendsScreen} />
        <SocialStack.Screen name="SearchFriends" component={SearchFriendsScreen} />
        <SocialStack.Screen name="FriendRequests" component={FriendRequestsScreen} />
        <SocialStack.Screen name="AdaptedInsights" component={AdaptedInsightsScreen} />
        <SocialStack.Screen name="AdaptedInsight" component={AdaptedInsightScreen} />
        <SocialStack.Screen name="ShareInsight" component={ShareInsightScreen} />
        <SocialStack.Screen name="SocialSettings" component={SocialSettingsScreen} />
        <SocialStack.Screen name="ActivityDetail" component={ActivityDetailScreen} />
      </SocialStack.Navigator>
    </TabSwipeWrapper>
  );
}

function CoachesStackScreen() {
  return (
    <TabSwipeWrapper tabName="CoachesTab">
      <CoachesStack.Navigator screenOptions={{ headerShown: false }}>
        <CoachesStack.Screen name="CoachesMain" component={CoachLibraryScreen} />
        <CoachesStack.Screen name="CoachDetail" component={CoachDetailScreen} />
        <CoachesStack.Screen name="CoachEditor" component={CoachEditorScreen} />
      </CoachesStack.Navigator>
    </TabSwipeWrapper>
  );
}

function DiscoverStackScreen() {
  return (
    <TabSwipeWrapper tabName="DiscoverTab">
      <DiscoverStack.Navigator screenOptions={{ headerShown: false }}>
        <DiscoverStack.Screen name="Store" component={StoreScreen} />
        <DiscoverStack.Screen name="StoreCoachDetail" component={StoreCoachDetailScreen} />
      </DiscoverStack.Navigator>
    </TabSwipeWrapper>
  );
}

function SettingsStackScreen() {
  return (
    <TabSwipeWrapper tabName="SettingsTab">
      <SettingsStack.Navigator screenOptions={{ headerShown: false }}>
        <SettingsStack.Screen name="SettingsMain" component={SettingsScreen} />
        <SettingsStack.Screen name="Connections" component={ConnectionsScreen} />
      </SettingsStack.Navigator>
    </TabSwipeWrapper>
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
    ChatTab: { icon: 'message-circle', label: 'Chat' },
    CoachesTab: { icon: 'award', label: 'Coaches' },
    DiscoverTab: { icon: 'compass', label: 'Discover' },
    SocialTab: { icon: 'zap', label: 'Insights' },
    SettingsTab: { icon: 'settings', label: 'Settings' },
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
            testID={`tab-${config.label.toLowerCase()}`}
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
        name="DiscoverTab"
        component={DiscoverStackScreen}
        options={{ tabBarLabel: 'Discover' }}
      />
      <Tab.Screen
        name="SocialTab"
        component={SocialStackScreen}
        options={{ tabBarLabel: 'Insights' }}
      />
      <Tab.Screen
        name="SettingsTab"
        component={SettingsStackScreen}
        options={{ tabBarLabel: 'Profile' }}
      />
    </Tab.Navigator>
  );
}

