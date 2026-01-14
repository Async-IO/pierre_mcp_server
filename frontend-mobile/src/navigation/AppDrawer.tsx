// ABOUTME: Main app drawer navigation for authenticated users
// ABOUTME: Contains Chat, Settings screens and recent conversations in drawer

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  SafeAreaView,
  ActivityIndicator,
  Alert,
  Modal,
  ScrollView,
  AppState,
} from 'react-native';
import * as Linking from 'expo-linking';
import * as WebBrowser from 'expo-web-browser';
import {
  createDrawerNavigator,
  type DrawerContentComponentProps,
} from '@react-navigation/drawer';
import { useFocusEffect } from '@react-navigation/native';
import { ChatScreen } from '../screens/chat/ChatScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { SettingsScreen } from '../screens/settings/SettingsScreen';
import { ConversationsScreen } from '../screens/conversations/ConversationsScreen';
import { useAuth } from '../contexts/AuthContext';
import { apiService } from '../services/api';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';
import type { Conversation, ProviderStatus } from '../types';

export type AppDrawerParamList = {
  Chat: { conversationId?: string } | undefined;
  Conversations: undefined;
  Connections: undefined;
  Settings: undefined;
};

const Drawer = createDrawerNavigator<AppDrawerParamList>();

function CustomDrawerContent(props: DrawerContentComponentProps) {
  const { user, isAuthenticated } = useAuth();
  const { navigation } = props;
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [isLoadingConversations, setIsLoadingConversations] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(null);
  const [providerModalVisible, setProviderModalVisible] = useState(false);
  const [connectedProviders, setConnectedProviders] = useState<ProviderStatus[]>([]);

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoadingConversations(true);
      const response = await apiService.getConversations();
      // Deduplicate by ID, sort by updated_at descending (most recent first), take top 10
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      const sorted = deduplicated
        .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
        .slice(0, 10);
      setConversations(sorted);
    } catch (error) {
      console.error('Failed to load conversations:', error);
    } finally {
      setIsLoadingConversations(false);
    }
  }, [isAuthenticated]);

  const loadProviderStatus = useCallback(async () => {
    if (!isAuthenticated) return;
    try {
      const response = await apiService.getOAuthStatus();
      setConnectedProviders(response.providers || []);
    } catch (error) {
      console.error('Failed to load provider status:', error);
    }
  }, [isAuthenticated]);

  // Load conversations and provider status when drawer is focused
  useFocusEffect(
    useCallback(() => {
      loadConversations();
      loadProviderStatus();
    }, [loadConversations, loadProviderStatus])
  );

  // Refresh provider status when app returns from OAuth flow
  useEffect(() => {
    const subscription = AppState.addEventListener('change', (nextAppState) => {
      if (nextAppState === 'active') {
        loadProviderStatus();
      }
    });
    return () => subscription.remove();
  }, [loadProviderStatus]);

  const handleConnectProvider = async (provider: string) => {
    setProviderModalVisible(false);
    try {
      const returnUrl = Linking.createURL('oauth-callback');
      const oauthResponse = await apiService.initMobileOAuth(provider, returnUrl);
      const result = await WebBrowser.openAuthSessionAsync(
        oauthResponse.authorization_url,
        returnUrl
      );

      if (result.type === 'success' && result.url) {
        const parsedUrl = Linking.parse(result.url);
        const success = parsedUrl.queryParams?.success === 'true';
        const error = parsedUrl.queryParams?.error as string | undefined;

        if (success) {
          await loadProviderStatus();
          Alert.alert('Connected', `Successfully connected to ${provider}!`);
        } else if (error) {
          Alert.alert('Connection Failed', `Failed to connect: ${error}`);
        } else {
          await loadProviderStatus();
        }
      }
    } catch (error) {
      console.error('Failed to start OAuth:', error);
      Alert.alert('Error', 'Failed to connect provider. Please try again.');
    }
  };

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

  const handleConversationLongPress = (conversation: Conversation) => {
    setSelectedConversation(conversation);
    setActionMenuVisible(true);
  };

  const handleNewChat = () => {
    navigation.navigate('Chat', { conversationId: undefined });
  };

  const handleRename = () => {
    if (!selectedConversation) return;
    setActionMenuVisible(false);

    Alert.prompt(
      'Rename Conversation',
      'Enter a new name for this conversation',
      async (newTitle: string | undefined) => {
        if (!newTitle?.trim() || !selectedConversation) return;
        try {
          const updated = await apiService.updateConversation(selectedConversation.id, {
            title: newTitle.trim(),
          });
          // Update conversation and move to top (most recently updated)
          setConversations(prev => {
            const updatedConv = prev.find(c => c.id === selectedConversation.id);
            if (!updatedConv) return prev;
            const others = prev.filter(c => c.id !== selectedConversation.id);
            return [
              { ...updatedConv, title: updated.title, updated_at: updated.updated_at },
              ...others,
            ];
          });
        } catch (error) {
          console.error('Failed to rename conversation:', error);
        }
      },
      'plain-text',
      selectedConversation.title || ''
    );
  };

  const handleDelete = () => {
    if (!selectedConversation) return;
    setActionMenuVisible(false);

    Alert.alert(
      'Delete Conversation',
      `Are you sure you want to delete "${selectedConversation.title || 'this conversation'}"?`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: async () => {
            try {
              await apiService.deleteConversation(selectedConversation.id);
              setConversations(prev => prev.filter(c => c.id !== selectedConversation.id));
            } catch (error) {
              console.error('Failed to delete conversation:', error);
            }
          },
        },
      ]
    );
  };

  const closeActionMenu = () => {
    setActionMenuVisible(false);
    setSelectedConversation(null);
  };

  return (
    <SafeAreaView style={styles.drawerContainer}>
      {/* Brand Header */}
      <View style={styles.brandHeader}>
        <Text style={styles.brandTitle}>Pierre</Text>
      </View>

      {/* Discussions Button */}
      <TouchableOpacity
        style={styles.discussionsButton}
        onPress={() => navigation.navigate('Conversations')}
      >
        <Text style={styles.discussionsIcon}>💬</Text>
        <Text style={styles.discussionsText}>Discussions</Text>
        <Text style={styles.discussionsChevron}>›</Text>
      </TouchableOpacity>

      {/* Connect Providers Button */}
      <TouchableOpacity
        style={styles.discussionsButton}
        onPress={() => setProviderModalVisible(true)}
      >
        <Text style={styles.discussionsIcon}>🔗</Text>
        <Text style={styles.discussionsText}>Connect Providers</Text>
        <Text style={styles.discussionsChevron}>›</Text>
      </TouchableOpacity>

      {/* Conversations List */}
      <ScrollView style={styles.conversationsContainer} contentContainerStyle={styles.conversationsContent}>
        {conversations.length > 0 && (
          <>
            <Text style={styles.sectionHeader}>Recents</Text>
            {conversations.map((conv) => (
              <TouchableOpacity
                key={conv.id}
                style={styles.conversationItem}
                onPress={() => handleConversationPress(conv.id)}
                onLongPress={() => handleConversationLongPress(conv)}
                delayLongPress={300}
              >
                <Text style={styles.conversationTitle} numberOfLines={1}>
                  {conv.title || 'Untitled'}
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

      {/* Floating Bottom Bar: Avatar + Username + New Chat */}
      <View style={styles.floatingBottomBar}>
        <TouchableOpacity
          style={styles.userButton}
          onPress={() => navigation.navigate('Settings')}
          activeOpacity={0.7}
        >
          <View style={styles.userAvatar}>
            <Text style={styles.userAvatarText}>
              {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
            </Text>
          </View>
          <Text style={styles.userName} numberOfLines={1}>
            {user?.display_name || 'User'}
          </Text>
        </TouchableOpacity>
        <TouchableOpacity style={styles.newChatButton} onPress={handleNewChat}>
          <Text style={styles.newChatIcon}>+</Text>
        </TouchableOpacity>
      </View>

      {/* Action Menu Modal - Floating popup style */}
      <Modal
        visible={actionMenuVisible}
        animationType="fade"
        transparent
        onRequestClose={closeActionMenu}
      >
        <TouchableOpacity
          style={styles.modalOverlay}
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View style={styles.actionMenuContainer}>
            <TouchableOpacity
              style={[styles.actionMenuItem, styles.actionMenuItemDisabled]}
              disabled
            >
              <Text style={styles.actionMenuIcon}>☆</Text>
              <Text style={styles.actionMenuTextDisabled}>Add to favorites</Text>
            </TouchableOpacity>

            <TouchableOpacity
              style={styles.actionMenuItem}
              onPress={handleRename}
            >
              <Text style={styles.actionMenuIcon}>✎</Text>
              <Text style={styles.actionMenuText}>Rename</Text>
            </TouchableOpacity>

            <TouchableOpacity
              style={styles.actionMenuItem}
              onPress={handleDelete}
            >
              <Text style={styles.actionMenuIconDanger}>🗑</Text>
              <Text style={styles.actionMenuTextDanger}>Delete</Text>
            </TouchableOpacity>
          </View>
        </TouchableOpacity>
      </Modal>

      {/* Provider Bottom Sheet Modal */}
      <Modal
        visible={providerModalVisible}
        animationType="slide"
        transparent
        onRequestClose={() => setProviderModalVisible(false)}
      >
        <TouchableOpacity
          style={styles.bottomSheetOverlay}
          activeOpacity={1}
          onPress={() => setProviderModalVisible(false)}
        >
          <View style={styles.bottomSheetContainer}>
            <View style={styles.bottomSheetHandle} />
            <Text style={styles.bottomSheetTitle}>Connect a Provider</Text>
            <Text style={styles.bottomSheetSubtitle}>
              Link your fitness accounts to analyze your data
            </Text>

            <ScrollView style={styles.providerList} showsVerticalScrollIndicator={false}>
              {[
                { id: 'strava', name: 'Strava', icon: '🚴' },
                { id: 'fitbit', name: 'Fitbit', icon: '⌚' },
                { id: 'garmin', name: 'Garmin', icon: '⌚' },
                { id: 'whoop', name: 'WHOOP', icon: '💪' },
                { id: 'coros', name: 'COROS', icon: '🏃' },
                { id: 'terra', name: 'Terra', icon: '🌍' },
              ].map((provider) => {
                const providers = Array.isArray(connectedProviders) ? connectedProviders : [];
                const providerStatus = providers.find(
                  (p) => p.provider === provider.id
                );
                const isAvailable = !!providerStatus;
                const isConnected = providerStatus?.connected ?? false;

                return (
                  <TouchableOpacity
                    key={provider.id}
                    style={[
                      styles.providerButton,
                      isConnected && styles.providerButtonConnected,
                      !isAvailable && styles.providerButtonDisabled,
                    ]}
                    onPress={() => handleConnectProvider(provider.id)}
                    disabled={!isAvailable}
                  >
                    <Text style={[
                      styles.providerIcon,
                      !isAvailable && styles.providerIconDisabled,
                    ]}>{provider.icon}</Text>
                    <Text style={[
                      styles.providerName,
                      !isAvailable && styles.providerNameDisabled,
                    ]}>{provider.name}</Text>
                    {isConnected && (
                      <Text style={styles.providerConnectedBadge}>✓</Text>
                    )}
                    {!isAvailable && (
                      <Text style={styles.providerComingSoon}>Coming soon</Text>
                    )}
                  </TouchableOpacity>
                );
              })}
            </ScrollView>

            <TouchableOpacity
              style={styles.bottomSheetCloseButton}
              onPress={() => setProviderModalVisible(false)}
            >
              <Text style={styles.bottomSheetCloseText}>Close</Text>
            </TouchableOpacity>
          </View>
        </TouchableOpacity>
      </Modal>
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
      <Drawer.Screen name="Conversations" component={ConversationsScreen} />
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
  brandHeader: {
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
    paddingBottom: spacing.md,
  },
  brandTitle: {
    fontSize: 28,
    fontWeight: '700',
    color: colors.text.primary,
  },
  discussionsButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    marginBottom: spacing.sm,
  },
  discussionsIcon: {
    fontSize: 18,
    marginRight: spacing.sm,
  },
  discussionsText: {
    flex: 1,
    fontSize: fontSize.md,
    fontWeight: '500',
    color: colors.text.primary,
  },
  discussionsChevron: {
    fontSize: 20,
    color: colors.text.tertiary,
  },
  sectionHeader: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.tertiary,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.xs,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  conversationsContainer: {
    flex: 1,
  },
  conversationsContent: {
    paddingBottom: 80, // Space for floating bottom bar
  },
  conversationItem: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm + 2,
  },
  conversationTitle: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  loadingContainer: {
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  floatingBottomBar: {
    position: 'absolute',
    bottom: spacing.lg,
    left: spacing.md,
    right: spacing.md,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  userButton: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.background.tertiary,
    paddingVertical: spacing.xs,
    paddingHorizontal: spacing.sm,
    borderRadius: borderRadius.full,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 4,
  },
  userAvatar: {
    width: 28,
    height: 28,
    borderRadius: 14,
    backgroundColor: colors.primary[700],
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.xs,
  },
  userAvatarText: {
    fontSize: 12,
    fontWeight: '600',
    color: colors.text.primary,
  },
  userName: {
    fontSize: fontSize.sm,
    fontWeight: '500',
    color: colors.text.primary,
    maxWidth: 120,
  },
  newChatButton: {
    width: 44,
    height: 44,
    borderRadius: 22,
    backgroundColor: colors.primary[500],
    alignItems: 'center',
    justifyContent: 'center',
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 4,
  },
  newChatIcon: {
    fontSize: 28,
    color: colors.text.primary,
    marginTop: -2,
  },
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.3)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  actionMenuContainer: {
    backgroundColor: colors.background.primary,
    borderRadius: borderRadius.lg,
    paddingVertical: spacing.xs,
    minWidth: 200,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 8,
  },
  actionMenuItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
  },
  actionMenuItemDisabled: {
    opacity: 0.4,
  },
  actionMenuIcon: {
    fontSize: 18,
    marginRight: spacing.sm,
    width: 24,
  },
  actionMenuIconDanger: {
    fontSize: 18,
    marginRight: spacing.sm,
    width: 24,
  },
  actionMenuText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  actionMenuTextDisabled: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
  },
  actionMenuTextDanger: {
    fontSize: fontSize.md,
    color: colors.error,
  },
  // Bottom Sheet Modal styles
  bottomSheetOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    justifyContent: 'flex-end',
  },
  bottomSheetContainer: {
    backgroundColor: colors.background.primary,
    borderTopLeftRadius: borderRadius.xl,
    borderTopRightRadius: borderRadius.xl,
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.sm,
    paddingBottom: spacing.xl,
    maxHeight: '70%',
  },
  bottomSheetHandle: {
    width: 40,
    height: 4,
    backgroundColor: colors.text.tertiary,
    borderRadius: 2,
    alignSelf: 'center',
    marginBottom: spacing.md,
  },
  bottomSheetTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
    marginBottom: spacing.xs,
  },
  bottomSheetSubtitle: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    textAlign: 'center',
    marginBottom: spacing.lg,
  },
  providerList: {
    maxHeight: 300,
  },
  providerButton: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    padding: spacing.md,
    marginBottom: spacing.sm,
    borderWidth: 1,
    borderColor: colors.border.default,
  },
  providerButtonConnected: {
    borderColor: colors.primary[500],
    backgroundColor: colors.background.tertiary,
  },
  providerButtonDisabled: {
    opacity: 0.5,
    borderColor: colors.border.subtle,
  },
  providerIcon: {
    fontSize: 24,
    marginRight: spacing.md,
  },
  providerIconDisabled: {
    opacity: 0.5,
  },
  providerName: {
    flex: 1,
    fontSize: fontSize.md,
    color: colors.text.primary,
    fontWeight: '500',
  },
  providerNameDisabled: {
    color: colors.text.tertiary,
  },
  providerConnectedBadge: {
    fontSize: 18,
    color: colors.primary[500],
    fontWeight: '600',
  },
  providerComingSoon: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    fontStyle: 'italic',
  },
  bottomSheetCloseButton: {
    alignItems: 'center',
    padding: spacing.md,
    marginTop: spacing.sm,
  },
  bottomSheetCloseText: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
    fontWeight: '500',
  },
});
