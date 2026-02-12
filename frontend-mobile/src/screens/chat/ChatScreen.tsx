// ABOUTME: Main chat screen orchestrator importing decomposed hooks and components
// ABOUTME: Coordinates conversation, message, provider, coach, and voice input state

import React, { useState, useRef, useEffect, useCallback } from 'react';
import { View, TextInput, KeyboardAvoidingView, Platform, Alert } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import * as Linking from 'expo-linking';
import Toast from 'react-native-toast-message';
import { useRoute, useFocusEffect, type RouteProp } from '@react-navigation/native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { BottomTabNavigationProp } from '@react-navigation/bottom-tabs';

import { useAuth } from '../../contexts/AuthContext';
import { PromptDialog } from '../../components/ui';
import { SharePreviewModal } from '../../components/social';
import { socialApi } from '../../services/api';
import type { ShareVisibility, Coach } from '../../types';
import type { ChatStackParamList, MainTabsParamList } from '../../navigation/MainTabs';

import { ChatHeader } from './ChatHeader';
import { ChatInputBar } from './ChatInputBar';
import { MessageList } from './MessageList';
import { ProviderModal } from './ProviderModal';
import { useConversations } from './useConversations';
import { useMessages } from './useMessages';
import { useProviderStatus } from './useProviderStatus';
import { useCoachSelection } from './useCoachSelection';
import { useChatVoiceInput } from './useChatVoiceInput';

interface ChatScreenProps {
  navigation: NativeStackNavigationProp<ChatStackParamList>;
}

export function ChatScreen({ navigation }: ChatScreenProps) {
  const { isAuthenticated } = useAuth();
  const insets = useSafeAreaInsets();
  const route = useRoute<RouteProp<ChatStackParamList, 'ChatMain'>>();
  const tabNavigation = useNavigation<BottomTabNavigationProp<MainTabsParamList>>();
  const inputRef = useRef<TextInput>(null);

  // UI State
  const [inputText, setInputText] = useState('');
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [renamePromptVisible, setRenamePromptVisible] = useState(false);
  const [renameConversationId, setRenameConversationId] = useState<string | null>(null);
  const [renameDefaultTitle, setRenameDefaultTitle] = useState('');
  const [shareToFeedContent, setShareToFeedContent] = useState<string | null>(null);
  const [shareToFeedVisibility, setShareToFeedVisibility] = useState<ShareVisibility>('friends_only');
  const [isSharing, setIsSharing] = useState(false);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);

  // Custom hooks
  const conversations = useConversations();
  const messagesHook = useMessages();
  const providerStatus = useProviderStatus();
  const coachSelection = useCoachSelection();

  // Voice input with chat-specific error handling
  const voiceInput = useChatVoiceInput(
    (text) => setInputText(text),
    setInputText
  );

  // Load data when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      conversations.loadConversations();
      providerStatus.loadProviderStatus();
      coachSelection.loadCoaches();
    }
    // These functions are stable from hooks, intentionally omit to avoid loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated]);

  // Refresh provider status on focus
  useFocusEffect(
    useCallback(() => {
      if (isAuthenticated) {
        providerStatus.loadProviderStatus();
      }
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [isAuthenticated])
  );

  // Load messages when conversation changes
  useEffect(() => {
    if (conversations.currentConversation) {
      if (conversations.justCreatedConversationRef.current === conversations.currentConversation.id) {
        conversations.justCreatedConversationRef.current = null;
        return;
      }
      messagesHook.loadMessages(conversations.currentConversation.id);
    } else {
      messagesHook.clearMessages();
    }
    // Intentionally only depend on currentConversation to avoid infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conversations.currentConversation]);

  // Handle navigation params for conversation selection
  useEffect(() => {
    const conversationId = route.params?.conversationId;
    if (conversationId === undefined && conversations.currentConversation !== null) {
      conversations.setCurrentConversation(null);
      messagesHook.clearMessages();
    }
    // Only depend on route params - this should only run when user navigates
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.params?.conversationId]);

  useEffect(() => {
    const conversationId = route.params?.conversationId;
    if (conversationId && conversations.conversations.length > 0) {
      const conversation = conversations.conversations.find(c => c.id === conversationId);
      const shouldUpdate = conversation && (
        conversation.id !== conversations.currentConversation?.id ||
        (!conversations.currentConversation?.title && conversation.title)
      );
      if (shouldUpdate) {
        conversations.setCurrentConversation(conversation);
      }
    }
    // currentConversation intentionally omitted - including it would cause infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.params?.conversationId, conversations.conversations]);

  // URL handling
  const handleOpenUrl = useCallback(async (url: string) => {
    try {
      let parsedUrl: URL;
      try {
        parsedUrl = new URL(url);
      } catch {
        console.error('Invalid URL:', url);
        Alert.alert('Error', 'Invalid link format');
        return;
      }

      const scheme = parsedUrl.protocol.toLowerCase();
      if (scheme !== 'http:' && scheme !== 'https:') {
        console.warn('Blocked non-HTTP URL scheme:', scheme);
        Alert.alert('Blocked', 'Only HTTP and HTTPS links can be opened.');
        return;
      }

      await Linking.openURL(url);
    } catch (error) {
      console.error('Failed to open URL:', error);
      Alert.alert('Error', 'Failed to open link');
    }
  }, []);

  // Message sending
  const handleSendMessage = useCallback(async () => {
    if (!inputText.trim() || messagesHook.isSending) return;

    const messageText = inputText.trim();
    setInputText('');

    let conversationId = conversations.currentConversation?.id;
    if (!conversationId) {
      const newConversation = await conversations.createConversation({
        title: messageText.slice(0, 50),
      });
      if (!newConversation) return;
      conversationId = newConversation.id;
    }

    await messagesHook.sendMessage(conversationId, messageText);
  }, [inputText, messagesHook, conversations]);

  // Insight creation
  const handleCreateInsight = useCallback(async (content: string) => {
    await messagesHook.createInsight(
      content,
      conversations.currentConversation?.id,
      async () => {
        const newConversation = await conversations.createConversation({
          title: 'Insight Generation',
        });
        return newConversation?.id || null;
      }
    );
  }, [messagesHook, conversations]);

  // Retry message
  const handleRetryMessage = useCallback(async (messageId: string) => {
    if (!conversations.currentConversation?.id) return;
    await messagesHook.retryMessage(messageId, conversations.currentConversation.id);
  }, [messagesHook, conversations.currentConversation?.id]);

  // Coach selection handling
  const handleCoachSelect = useCallback(async (coach: Coach) => {
    await coachSelection.handleCoachSelect(coach, {
      isSending: messagesHook.isSending,
      hasConnectedProvider: providerStatus.hasConnectedProvider,
      selectedProvider: providerStatus.selectedProvider,
      connectedProviders: providerStatus.connectedProviders,
      setSelectedProvider: providerStatus.setSelectedProvider,
      setProviderModalVisible: providerStatus.setProviderModalVisible,
      startCoachConversation: async (coach) => {
        await coachSelection.startCoachConversation(coach, {
          createConversation: conversations.createConversation,
          setMessages: (msgs) => {
            // Handle both function and value updates
            if (typeof msgs === 'function') {
              messagesHook.clearMessages();
            }
          },
          setIsSending: () => {},
          scrollToBottom: messagesHook.scrollToBottom,
        });
      },
    });
  }, [coachSelection, messagesHook, providerStatus, conversations]);

  // Start coach conversation helper
  const startCoachConversation = useCallback(async (coach: Coach) => {
    await coachSelection.startCoachConversation(coach, {
      createConversation: conversations.createConversation,
      setMessages: () => messagesHook.clearMessages(),
      setIsSending: () => {},
      scrollToBottom: messagesHook.scrollToBottom,
    });
  }, [coachSelection, conversations, messagesHook]);

  // Provider connection handling
  const handleConnectProvider = useCallback(async (provider: string) => {
    await providerStatus.handleConnectProvider(provider, async () => {
      if (coachSelection.pendingCoachAction) {
        await startCoachConversation(coachSelection.pendingCoachAction.coach);
        coachSelection.clearPendingCoachAction();
      } else if (pendingPrompt) {
        await handleSendPromptMessage(pendingPrompt);
        setPendingPrompt(null);
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providerStatus, coachSelection, pendingPrompt, startCoachConversation]);

  const handleSendPromptMessage = useCallback(async (prompt: string) => {
    let conversationId = conversations.currentConversation?.id;
    if (!conversationId) {
      const newConversation = await conversations.createConversation({
        title: prompt.slice(0, 50),
      });
      if (!newConversation) return;
      conversationId = newConversation.id;
    }
    await messagesHook.sendMessage(conversationId, prompt);
  }, [conversations, messagesHook]);

  // Provider modal handlers
  const handleProviderSelect = useCallback((provider: string) => {
    providerStatus.setSelectedProvider(provider);
    providerStatus.setProviderModalVisible(false);
    if (pendingPrompt) {
      handleSendPromptMessage(pendingPrompt);
      setPendingPrompt(null);
    }
    if (coachSelection.pendingCoachAction) {
      startCoachConversation(coachSelection.pendingCoachAction.coach);
      coachSelection.clearPendingCoachAction();
    }
  }, [providerStatus, pendingPrompt, coachSelection, handleSendPromptMessage, startCoachConversation]);

  const handleProviderModalClose = useCallback(() => {
    providerStatus.setProviderModalVisible(false);
    setPendingPrompt(null);
    coachSelection.clearPendingCoachAction();
  }, [providerStatus, coachSelection]);

  // Share to feed
  const handleShareToFeed = useCallback((content: string) => {
    setShareToFeedContent(content);
  }, []);

  const handleShareToFeedSubmit = useCallback(async () => {
    if (!shareToFeedContent) return;

    setIsSharing(true);
    try {
      await socialApi.shareFromActivity({
        content: shareToFeedContent,
        insight_type: 'coaching_insight',
        visibility: shareToFeedVisibility,
      });
      Toast.show({
        type: 'success',
        text1: 'Shared to Social Feed',
        text2: 'Your insight has been posted',
      });
      setShareToFeedContent(null);
      setShareToFeedVisibility('friends_only');
      tabNavigation.navigate('SocialTab', { screen: 'SocialMain' } as never);
    } catch (error) {
      console.error('Failed to share to feed:', error);
      Toast.show({
        type: 'error',
        text1: 'Share Failed',
        text2: 'Could not share to social feed',
      });
    } finally {
      setIsSharing(false);
    }
  }, [shareToFeedContent, shareToFeedVisibility, tabNavigation]);

  const handleEditShare = useCallback(() => {
    if (!shareToFeedContent) return;
    const contentToEdit = shareToFeedContent;
    const visibilityToEdit = shareToFeedVisibility;
    setShareToFeedContent(null);
    setShareToFeedVisibility('friends_only');
    tabNavigation.navigate('SocialTab', {
      screen: 'ShareInsight',
      params: {
        content: contentToEdit,
        insightType: 'coaching_insight',
        visibility: visibilityToEdit,
      },
    } as never);
  }, [shareToFeedContent, shareToFeedVisibility, tabNavigation]);

  const handleCloseShareModal = useCallback(() => {
    setShareToFeedContent(null);
    setShareToFeedVisibility('friends_only');
  }, []);

  // Header menu handlers
  const showTitleActionMenu = useCallback(() => {
    if (!conversations.currentConversation) return;
    setActionMenuVisible(true);
  }, [conversations.currentConversation]);

  const handleMenuRename = useCallback(() => {
    setActionMenuVisible(false);
    if (conversations.currentConversation) {
      const title = conversations.currentConversation.title || 'New Chat';
      setRenameConversationId(conversations.currentConversation.id);
      setRenameDefaultTitle(title);
      setRenamePromptVisible(true);
    }
  }, [conversations.currentConversation]);

  const handleMenuDelete = useCallback(() => {
    setActionMenuVisible(false);
    if (!conversations.currentConversation) return;

    Alert.alert(
      'Delete Conversation',
      `Are you sure you want to delete "${conversations.currentConversation.title || 'this conversation'}"?`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: () => conversations.deleteConversation(conversations.currentConversation!.id),
        },
      ]
    );
  }, [conversations]);

  const handleRenameSubmit = useCallback(async (newTitle: string) => {
    setRenamePromptVisible(false);
    if (!renameConversationId) return;
    await conversations.renameConversation(renameConversationId, newTitle);
    setRenameConversationId(null);
    setRenameDefaultTitle('');
  }, [renameConversationId, conversations]);

  const handleRenameCancel = useCallback(() => {
    setRenamePromptVisible(false);
    setRenameConversationId(null);
    setRenameDefaultTitle('');
  }, []);

  const handleNewChat = useCallback(() => {
    conversations.handleNewChat();
    messagesHook.clearMessages();
  }, [conversations, messagesHook]);

  const isCoachConversation = Boolean(conversations.currentConversation?.system_prompt);

  return (
    <View className="flex-1 bg-background-primary" testID="chat-screen">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
        keyboardVerticalOffset={Platform.OS === 'ios' ? 0 : 0}
      >
        <ChatHeader
          currentConversation={conversations.currentConversation}
          actionMenuVisible={actionMenuVisible}
          insetTop={insets.top}
          onBackPress={handleNewChat}
          onHistoryPress={() => navigation.navigate('Conversations')}
          onTitlePress={showTitleActionMenu}
          onNewChatPress={handleNewChat}
          onMenuClose={() => setActionMenuVisible(false)}
          onMenuRename={handleMenuRename}
          onMenuDelete={handleMenuDelete}
        />

        <MessageList
          messages={messagesHook.messages}
          coaches={coachSelection.coaches}
          isLoading={conversations.isLoading}
          isSending={messagesHook.isSending}
          isCoachConversation={isCoachConversation}
          messageFeedback={messagesHook.messageFeedback}
          insightMessages={messagesHook.insightMessages}
          flatListRef={messagesHook.flatListRef}
          onScrollToBottom={messagesHook.scrollToBottom}
          onCoachSelect={handleCoachSelect}
          onCreateInsight={handleCreateInsight}
          onShareToFeed={handleShareToFeed}
          onThumbsUp={messagesHook.handleThumbsUp}
          onThumbsDown={messagesHook.handleThumbsDown}
          onRetryMessage={handleRetryMessage}
          onOpenUrl={handleOpenUrl}
        />

        <ChatInputBar
          inputText={inputText}
          partialTranscript={voiceInput.partialTranscript}
          isListening={voiceInput.isListening}
          isSending={messagesHook.isSending}
          voiceAvailable={voiceInput.isAvailable}
          insetBottom={insets.bottom}
          inputRef={inputRef}
          onChangeText={setInputText}
          onVoicePress={voiceInput.handleVoicePress}
          onSendMessage={handleSendMessage}
        />

        <ProviderModal
          visible={providerStatus.providerModalVisible}
          providers={providerStatus.connectedProviders}
          onClose={handleProviderModalClose}
          onSelectConnected={handleProviderSelect}
          onConnectProvider={handleConnectProvider}
        />

        <PromptDialog
          visible={renamePromptVisible}
          title="Rename Chat"
          message="Enter a new name for this conversation"
          defaultValue={renameDefaultTitle}
          submitText="Save"
          cancelText="Cancel"
          onSubmit={handleRenameSubmit}
          onCancel={handleRenameCancel}
          testID="rename-conversation-dialog"
        />

        <SharePreviewModal
          visible={shareToFeedContent !== null}
          content={shareToFeedContent || ''}
          visibility={shareToFeedVisibility}
          isSharing={isSharing}
          onVisibilityChange={setShareToFeedVisibility}
          onShare={handleShareToFeedSubmit}
          onEdit={handleEditShare}
          onClose={handleCloseShareModal}
        />
      </KeyboardAvoidingView>
    </View>
  );
}
