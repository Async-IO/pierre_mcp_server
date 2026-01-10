// ABOUTME: Main app drawer navigation for authenticated users
// ABOUTME: Contains Chat, Connections, and Settings screens with custom drawer content

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  ScrollView,
  SafeAreaView,
  Image,
  ActivityIndicator,
} from 'react-native';
import {
  createDrawerNavigator,
  type DrawerContentComponentProps,
} from '@react-navigation/drawer';
import { useFocusEffect } from '@react-navigation/native';
import { ChatScreen } from '../screens/chat/ChatScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { SettingsScreen } from '../screens/settings/SettingsScreen';
import { useAuth } from '../contexts/AuthContext';
import { apiService } from '../services/api';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';
import type { Conversation } from '../types';

export type AppDrawerParamList = {
  Chat: { conversationId?: string } | undefined;
  Connections: undefined;
  Settings: undefined;
};

const Drawer = createDrawerNavigator<AppDrawerParamList>();

function CustomDrawerContent(props: DrawerContentComponentProps) {
  const { user, isAuthenticated } = useAuth();
  const { navigation, state } = props;
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [isLoadingConversations, setIsLoadingConversations] = useState(false);

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoadingConversations(true);
      const response = await apiService.getConversations();
      // Sort by updated_at descending (most recent first) and take top 10
      const sorted = (response.conversations || [])
        .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
        .slice(0, 10);
      setConversations(sorted);
    } catch (error) {
      console.error('Failed to load conversations:', error);
    } finally {
      setIsLoadingConversations(false);
    }
  }, [isAuthenticated]);

  // Load conversations when drawer is focused
  useFocusEffect(
    useCallback(() => {
      loadConversations();
    }, [loadConversations])
  );

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } else if (diffDays === 1) {
      return 'Yesterday';
    } else if (diffDays < 7) {
      return date.toLocaleDateString([], { weekday: 'short' });
    } else {
      return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
    }
  };

  const handleConversationPress = (conversationId: string) => {
    navigation.navigate('Chat', { conversationId });
  };

  const handleNewChat = () => {
    navigation.navigate('Chat', { conversationId: undefined });
  };

  const menuItems = [
    { name: 'Chat', icon: '💬', label: 'New Chat', action: handleNewChat },
    { name: 'Connections', icon: '🔗', label: 'Connections' },
    { name: 'Settings', icon: '⚙️', label: 'Settings' },
  ];

  return (
    <SafeAreaView style={styles.drawerContainer}>
      {/* Header */}
      <View style={styles.drawerHeader}>
        <Image
          source={require('../../assets/pierre-logo.png')}
          style={styles.drawerLogo}
          resizeMode="contain"
        />
        <Text style={styles.appName}>Pierre</Text>
      </View>

      {/* User Info */}
      <View style={styles.userSection}>
        <View style={styles.userAvatar}>
          <Text style={styles.userAvatarText}>
            {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
          </Text>
        </View>
        <View style={styles.userInfo}>
          <Text style={styles.userName} numberOfLines={1}>
            {user?.display_name || 'User'}
          </Text>
          <Text style={styles.userEmail} numberOfLines={1}>
            {user?.email}
          </Text>
        </View>
      </View>

      {/* Menu Items */}
      <ScrollView style={styles.menuContainer}>
        {menuItems.map((item) => {
          const isActive = state.routeNames[state.index] === item.name && !item.action;

          return (
            <TouchableOpacity
              key={item.name}
              style={[styles.menuItem, isActive && styles.menuItemActive]}
              onPress={item.action || (() => navigation.navigate(item.name as keyof AppDrawerParamList))}
            >
              <Text style={[styles.menuIcon, isActive && styles.menuTextActive]}>
                {item.icon}
              </Text>
              <Text style={[styles.menuLabel, isActive && styles.menuTextActive]}>
                {item.label}
              </Text>
            </TouchableOpacity>
          );
        })}

        {/* Recent Conversations */}
        {conversations.length > 0 && (
          <>
            <View style={styles.sectionHeader}>
              <Text style={styles.sectionTitle}>Recent Conversations</Text>
            </View>
            {conversations.map((conv) => (
              <TouchableOpacity
                key={conv.id}
                style={styles.conversationItem}
                onPress={() => handleConversationPress(conv.id)}
              >
                <Text style={styles.conversationTitle} numberOfLines={1}>
                  {conv.title || 'Untitled'}
                </Text>
                <Text style={styles.conversationDate}>
                  {formatDate(conv.updated_at)}
                </Text>
              </TouchableOpacity>
            ))}
          </>
        )}

        {isLoadingConversations && (
          <View style={styles.loadingContainer}>
            <ActivityIndicator size="small" color={colors.primary[500]} />
          </View>
        )}
      </ScrollView>

      {/* Footer */}
      <View style={styles.drawerFooter}>
        <Text style={styles.footerText}>Pierre v1.0.0</Text>
      </View>
    </SafeAreaView>
  );
}

export function AppDrawer() {
  return (
    <Drawer.Navigator
      drawerContent={(props) => <CustomDrawerContent {...props} />}
      screenOptions={{
        headerShown: false,
        drawerStyle: {
          backgroundColor: colors.background.secondary,
          width: 280,
        },
        drawerType: 'front',
        overlayColor: 'rgba(0, 0, 0, 0.5)',
      }}
    >
      <Drawer.Screen name="Chat" component={ChatScreen} />
      <Drawer.Screen name="Connections" component={ConnectionsScreen} />
      <Drawer.Screen name="Settings" component={SettingsScreen} />
    </Drawer.Navigator>
  );
}

const styles = StyleSheet.create({
  drawerContainer: {
    flex: 1,
    backgroundColor: colors.background.secondary,
  },
  drawerHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
    paddingBottom: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  drawerLogo: {
    width: 40,
    height: 40,
    marginRight: spacing.sm,
  },
  appName: {
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
  },
  userSection: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  userAvatar: {
    width: 44,
    height: 44,
    borderRadius: 22,
    backgroundColor: colors.primary[700],
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.sm,
  },
  userAvatarText: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text.primary,
  },
  userInfo: {
    flex: 1,
  },
  userName: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  userEmail: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  menuContainer: {
    flex: 1,
    paddingTop: spacing.md,
  },
  menuItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    marginHorizontal: spacing.sm,
    borderRadius: borderRadius.md,
  },
  menuItemActive: {
    backgroundColor: colors.primary[600] + '20',
  },
  menuIcon: {
    fontSize: 20,
    marginRight: spacing.md,
    color: colors.text.secondary,
  },
  menuLabel: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
  },
  menuTextActive: {
    color: colors.primary[500],
  },
  sectionHeader: {
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
    paddingBottom: spacing.xs,
  },
  sectionTitle: {
    fontSize: fontSize.xs,
    fontWeight: '600',
    color: colors.text.tertiary,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  conversationItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
    marginHorizontal: spacing.sm,
    borderRadius: borderRadius.md,
  },
  conversationTitle: {
    flex: 1,
    fontSize: fontSize.sm,
    color: colors.text.primary,
    marginRight: spacing.sm,
  },
  conversationDate: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
  },
  loadingContainer: {
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  drawerFooter: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  footerText: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    textAlign: 'center',
  },
});
