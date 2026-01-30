// ABOUTME: Main app drawer navigation for authenticated users
// ABOUTME: Provides drawer for conversations/providers with bottom tabs for primary screens

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  Modal,
  ScrollView,
  AppState,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import * as Linking from 'expo-linking';
import * as WebBrowser from 'expo-web-browser';
import { getOAuthCallbackUrl } from '../utils/oauth';
import {
  createDrawerNavigator,
  type DrawerContentComponentProps,
} from '@react-navigation/drawer';
import { useFocusEffect } from '@react-navigation/native';
import { MainTabs } from './MainTabs';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { ConversationsScreen } from '../screens/conversations/ConversationsScreen';
import { CoachEditorScreen } from '../screens/coaches/CoachEditorScreen';
import { CoachWizardScreen } from '../screens/coaches/CoachWizardScreen';
import { CoachDetailScreen } from '../screens/coaches/CoachDetailScreen';
import { StoreScreen } from '../screens/store/StoreScreen';
import { StoreCoachDetailScreen } from '../screens/store/StoreCoachDetailScreen';
import { FriendsScreen } from '../screens/social/FriendsScreen';
import { SearchFriendsScreen } from '../screens/social/SearchFriendsScreen';
import { FriendRequestsScreen } from '../screens/social/FriendRequestsScreen';
import { ShareInsightScreen } from '../screens/social/ShareInsightScreen';
import { AdaptedInsightScreen } from '../screens/social/AdaptedInsightScreen';
import { AdaptedInsightsScreen } from '../screens/social/AdaptedInsightsScreen';
import { SocialSettingsScreen } from '../screens/social/SocialSettingsScreen';
import { useAuth } from '../contexts/AuthContext';
import { apiService } from '../services/api';
import { colors, spacing, borderRadius } from '../constants/theme';
import { Feather } from '@expo/vector-icons';
import { PromptDialog } from '../components/ui';
import type { Conversation, ExtendedProviderStatus, AdaptedInsight } from '../types';

// Shadow styles for elevated elements (React Native requires style objects for shadows)
const userButtonShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.25,
  shadowRadius: 4,
  elevation: 4,
};

const actionMenuShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.3,
  shadowRadius: 8,
  elevation: 8,
};

export type AppDrawerParamList = {
  Main: undefined;
  Chat: { conversationId?: string } | undefined;
  Conversations: undefined;
  Connections: undefined;
  Settings: undefined;
  CoachLibrary: undefined;
  CoachEditor: { coachId?: string } | undefined;
  CoachWizard: { coachId?: string } | undefined;
  CoachDetail: { coachId: string };
  Store: undefined;
  StoreCoachDetail: { coachId: string };
  Friends: undefined;
  SearchFriends: undefined;
  FriendRequests: undefined;
  SocialFeed: undefined;
  ShareInsight: undefined;
  AdaptedInsight: { adaptedInsight: AdaptedInsight };
  AdaptedInsights: undefined;
  SocialSettings: undefined;
};

const Drawer = createDrawerNavigator<AppDrawerParamList>();

function CustomDrawerContent(props: DrawerContentComponentProps) {
  const { user, isAuthenticated } = useAuth();
  const insets = useSafeAreaInsets();
  const { navigation } = props;
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [isLoadingConversations, setIsLoadingConversations] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(null);
  const [providerModalVisible, setProviderModalVisible] = useState(false);
  const [connectedProviders, setConnectedProviders] = useState<ExtendedProviderStatus[]>([]);
  const [renamePromptVisible, setRenamePromptVisible] = useState(false);

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoadingConversations(true);
      const response = await apiService.getConversations();
      // Deduplicate by ID, sort by updated_at descending (most recent first), take top 10
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv: { id: string }) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      const sorted = deduplicated
        .sort((a: { updated_at: string }, b: { updated_at: string }) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
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
      const response = await apiService.getProvidersStatus();
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
      const returnUrl = getOAuthCallbackUrl();
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
    setRenamePromptVisible(true);
  };

  const handleRenameSubmit = async (newTitle: string) => {
    setRenamePromptVisible(false);
    if (!selectedConversation) return;

    try {
      const updated = await apiService.updateConversation(selectedConversation.id, {
        title: newTitle,
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
    } finally {
      setSelectedConversation(null);
    }
  };

  const handleRenameCancel = () => {
    setRenamePromptVisible(false);
    setSelectedConversation(null);
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
    <View
      className="flex-1 bg-background-secondary"
      style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
    >
      {/* Brand Header */}
      <View className="px-5 pt-5 pb-3">
        <Text className="text-[28px] font-bold text-text-primary">Pierre</Text>
      </View>

      {/* Quick Actions */}
      <TouchableOpacity
        className="flex-row items-center px-5 py-3 mb-2"
        onPress={() => navigation.navigate('Conversations')}
      >
        <Feather name="message-circle" size={20} color={colors.text.secondary} className="mr-2" />
        <Text className="flex-1 text-base font-medium text-text-primary">All Conversations</Text>
        <Feather name="chevron-right" size={18} color={colors.text.tertiary} />
      </TouchableOpacity>

      <TouchableOpacity
        className="flex-row items-center px-5 py-3 mb-2"
        onPress={() => setProviderModalVisible(true)}
      >
        <Feather name="link" size={20} color={colors.text.secondary} className="mr-2" />
        <Text className="flex-1 text-base font-medium text-text-primary">Connect Providers</Text>
        <Feather name="chevron-right" size={18} color={colors.text.tertiary} />
      </TouchableOpacity>

      <TouchableOpacity
        className="flex-row items-center px-5 py-3 mb-2"
        onPress={() => {
          // Navigate to Coaches tab at root (CoachesMain / My Coaches)
          navigation.navigate('Main', {
            screen: 'CoachesTab',
          });
        }}
      >
        <Feather name="book" size={20} color={colors.text.secondary} className="mr-2" />
        <Text className="flex-1 text-base font-medium text-text-primary">My Coaches</Text>
        <Feather name="chevron-right" size={18} color={colors.text.tertiary} />
      </TouchableOpacity>

      <TouchableOpacity
        className="flex-row items-center px-5 py-3 mb-2"
        onPress={() => {
          // Navigate to Coaches tab, then to Store screen within that tab's stack
          navigation.navigate('Main');
          // Use setTimeout to ensure tab is focused before navigating within its stack
          setTimeout(() => {
            navigation.navigate('Main', {
              screen: 'CoachesTab',
              params: { screen: 'Store' },
            });
          }, 100);
        }}
      >
        <Feather name="compass" size={20} color={colors.text.secondary} className="mr-2" />
        <Text className="flex-1 text-base font-medium text-text-primary">Discover Coaches</Text>
        <Feather name="chevron-right" size={18} color={colors.text.tertiary} />
      </TouchableOpacity>

      {/* Conversations List */}
      <ScrollView className="flex-1" contentContainerStyle={{ paddingBottom: 80 }}>
        {conversations.length > 0 && (
          <>
            <Text className="text-sm font-semibold text-text-tertiary px-5 py-1 uppercase tracking-wide">
              Recents
            </Text>
            {conversations.map((conv) => (
              <TouchableOpacity
                key={conv.id}
                className="px-5 py-2.5"
                onPress={() => handleConversationPress(conv.id)}
                onLongPress={() => handleConversationLongPress(conv)}
                delayLongPress={300}
              >
                <Text className="text-base text-text-primary" numberOfLines={1}>
                  {conv.title || 'Untitled'}
                </Text>
              </TouchableOpacity>
            ))}
          </>
        )}

        {isLoadingConversations && (
          <View className="py-3 items-center">
            <ActivityIndicator size="small" color={colors.primary[500]} />
          </View>
        )}
      </ScrollView>

      {/* Floating Bottom Bar: Avatar + Username + New Chat */}
      <View
        className="absolute flex-row items-center justify-between"
        style={{ bottom: spacing.lg, left: spacing.md, right: spacing.md }}
      >
        <TouchableOpacity
          className="flex-row items-center bg-background-tertiary py-1 px-2 rounded-full"
          style={userButtonShadow}
          onPress={() => navigation.navigate('Settings')}
          activeOpacity={0.7}
        >
          <View className="w-7 h-7 rounded-full bg-primary-700 items-center justify-center mr-1">
            <Text className="text-xs font-semibold text-text-primary">
              {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
            </Text>
          </View>
          <Text className="text-sm font-medium text-text-primary max-w-[120px]" numberOfLines={1}>
            {user?.display_name || 'User'}
          </Text>
        </TouchableOpacity>
        <TouchableOpacity
          className="w-11 h-11 rounded-full bg-primary-500 items-center justify-center"
          style={userButtonShadow}
          onPress={handleNewChat}
        >
          <Text className="text-[28px] text-text-primary -mt-0.5">+</Text>
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
          className="flex-1 bg-black/30 justify-center items-center"
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View
            className="bg-background-primary rounded-lg py-1 min-w-[200px]"
            style={actionMenuShadow}
          >
            <TouchableOpacity
              className="flex-row items-center px-4 py-2 opacity-40"
              disabled
            >
              <Text className="text-lg mr-2 w-6">☆</Text>
              <Text className="text-base text-text-tertiary">Add to favorites</Text>
            </TouchableOpacity>

            <TouchableOpacity
              className="flex-row items-center px-4 py-2"
              onPress={handleRename}
            >
              <Text className="text-lg mr-2 w-6">✎</Text>
              <Text className="text-base text-text-primary">Rename</Text>
            </TouchableOpacity>

            <TouchableOpacity
              className="flex-row items-center px-4 py-2"
              onPress={handleDelete}
            >
              <Text className="text-lg mr-2 w-6">🗑</Text>
              <Text className="text-base text-error">Delete</Text>
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
          className="flex-1 bg-black/50 justify-end"
          activeOpacity={1}
          onPress={() => setProviderModalVisible(false)}
        >
          <View
            className="bg-background-primary px-5 pt-2 pb-6 max-h-[70%]"
            style={{ borderTopLeftRadius: borderRadius.xl, borderTopRightRadius: borderRadius.xl }}
          >
            <View className="w-10 h-1 bg-text-tertiary rounded-full self-center mb-3" />
            <Text className="text-lg font-semibold text-text-primary text-center mb-1">
              Connect a Provider
            </Text>
            <Text className="text-sm text-text-secondary text-center mb-5">
              Link your fitness accounts to analyze your data
            </Text>

            <ScrollView style={{ maxHeight: 300 }} showsVerticalScrollIndicator={false}>
              {connectedProviders.map((provider) => {
                const PROVIDER_ICONS: Record<string, string> = {
                  strava: '🚴',
                  fitbit: '⌚',
                  garmin: '⌚',
                  whoop: '💪',
                  coros: '🏃',
                  terra: '🌍',
                  synthetic: '🧪',
                  synthetic_sleep: '😴',
                };
                const icon = PROVIDER_ICONS[provider.provider] || '🔗';
                const displayName = provider.display_name || provider.provider;
                const isConnected = provider.connected;
                const requiresOAuth = provider.requires_oauth;

                return (
                  <TouchableOpacity
                    key={provider.provider}
                    className={`flex-row items-center p-3 mb-2 rounded-lg border ${
                      isConnected
                        ? 'border-primary-500 bg-background-tertiary'
                        : 'border-border-default bg-background-secondary'
                    }`}
                    onPress={() => {
                      if (!isConnected && requiresOAuth) {
                        handleConnectProvider(provider.provider);
                      } else if (isConnected) {
                        setProviderModalVisible(false);
                      }
                    }}
                    disabled={!isConnected && !requiresOAuth}
                  >
                    <Text className="text-2xl mr-3">{icon}</Text>
                    <View className="flex-1">
                      <Text className="text-base font-medium text-text-primary">
                        {displayName}
                      </Text>
                      {isConnected && (
                        <Text className="text-xs text-primary-500">Connected</Text>
                      )}
                    </View>
                    {isConnected && (
                      <Text className="text-lg text-primary-500 font-semibold">✓</Text>
                    )}
                  </TouchableOpacity>
                );
              })}
            </ScrollView>

            <TouchableOpacity
              className="items-center p-3 mt-2"
              onPress={() => setProviderModalVisible(false)}
            >
              <Text className="text-base text-text-tertiary font-medium">Close</Text>
            </TouchableOpacity>
          </View>
        </TouchableOpacity>
      </Modal>

      {/* Rename Conversation Prompt Dialog */}
      <PromptDialog
        visible={renamePromptVisible}
        title="Rename Conversation"
        message="Enter a new name for this conversation"
        defaultValue={selectedConversation?.title || ''}
        submitText="Save"
        cancelText="Cancel"
        onSubmit={handleRenameSubmit}
        onCancel={handleRenameCancel}
        testID="rename-conversation-dialog"
      />
    </View>
  );
}

// Wrapper to pass params to ChatTab within MainTabs
function ChatScreenWrapper() {
  return <MainTabs />;
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
      <Drawer.Screen name="Main" component={MainTabs} />
      <Drawer.Screen name="Chat" component={ChatScreenWrapper} />
      <Drawer.Screen name="Conversations" component={ConversationsScreen} />
      <Drawer.Screen name="Connections" component={ConnectionsScreen} />
      <Drawer.Screen name="CoachEditor" component={CoachEditorScreen} />
      <Drawer.Screen name="CoachWizard" component={CoachWizardScreen} />
      <Drawer.Screen name="CoachDetail" component={CoachDetailScreen} />
      <Drawer.Screen name="Store" component={StoreScreen} />
      <Drawer.Screen name="StoreCoachDetail" component={StoreCoachDetailScreen} />
      <Drawer.Screen name="Friends" component={FriendsScreen} />
      <Drawer.Screen name="SearchFriends" component={SearchFriendsScreen} />
      <Drawer.Screen name="FriendRequests" component={FriendRequestsScreen} />
      <Drawer.Screen name="ShareInsight" component={ShareInsightScreen} />
      <Drawer.Screen name="AdaptedInsight" component={AdaptedInsightScreen} />
      <Drawer.Screen name="AdaptedInsights" component={AdaptedInsightsScreen} />
      <Drawer.Screen name="SocialSettings" component={SocialSettingsScreen} />
    </Drawer.Navigator>
  );
}

