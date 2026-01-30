// ABOUTME: Main chat screen with conversation list and message interface
// ABOUTME: Professional dark theme UI inspired by ChatGPT and Claude design

import React, { useState, useRef, useEffect } from 'react';
import {
  View,
  Text,
  FlatList,
  TextInput,
  TouchableOpacity,
  KeyboardAvoidingView,
  Platform,
  ActivityIndicator,
  Alert,
  ScrollView,
  Modal,
  Share,
  AppState,
  Image,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import * as Clipboard from 'expo-clipboard';
import * as Linking from 'expo-linking';
import * as WebBrowser from 'expo-web-browser';
import * as Haptics from 'expo-haptics';
import Toast from 'react-native-toast-message';
import { getOAuthCallbackUrl } from '../../utils/oauth';
import Markdown from 'react-native-markdown-display';
import { Ionicons } from '@expo/vector-icons';
import { useRoute, type RouteProp } from '@react-navigation/native';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';

// ViewStyle objects for styles that require React Native shadow properties
const popoverContainerStyle: ViewStyle = {
  position: 'absolute',
  top: 68,
  left: 60,
  right: 60,
  backgroundColor: colors.background.secondary,
  borderRadius: 12,
  paddingVertical: spacing.xs,
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 8 },
  shadowOpacity: 0.4,
  shadowRadius: 16,
  elevation: 12,
};

const providerModalContainerStyle: ViewStyle = {
  backgroundColor: colors.background.primary,
  borderRadius: borderRadius.lg,
  padding: spacing.lg,
  minWidth: 280,
  maxWidth: 320,
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.3,
  shadowRadius: 8,
  elevation: 8,
};
import { chatApi, coachesApi, oauthApi, socialApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { useVoiceInput } from '../../hooks/useVoiceInput';
import type { VoiceError } from '../../hooks/useVoiceInput';
import { VoiceButton, PromptDialog } from '../../components/ui';
import type { Conversation, Message, ExtendedProviderStatus, Coach } from '../../types';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { ChatStackParamList } from '../../navigation/MainTabs';

// Coach category badge background colors (lighter versions)
const COACH_CATEGORY_BADGE_BG: Record<string, string> = {
  training: 'rgba(16, 185, 129, 0.15)',
  nutrition: 'rgba(245, 158, 11, 0.15)',
  recovery: 'rgba(99, 102, 241, 0.15)',
  recipes: 'rgba(249, 115, 22, 0.15)',
  mobility: 'rgba(236, 72, 153, 0.15)',
  custom: 'rgba(124, 58, 237, 0.15)',
};

// Coach category emoji icons
const COACH_CATEGORY_ICONS: Record<string, string> = {
  training: '🏃',
  nutrition: '🥗',
  recovery: '😴',
  recipes: '👨‍🍳',
  mobility: '🧘',
  custom: '⚙️',
};

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

interface ChatScreenProps {
  navigation: NativeStackNavigationProp<ChatStackParamList>;
}

export function ChatScreen({ navigation }: ChatScreenProps) {
  const { isAuthenticated } = useAuth();
  const insets = useSafeAreaInsets();
  const route = useRoute<RouteProp<ChatStackParamList, 'ChatMain'>>();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversation, setCurrentConversation] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [providerModalVisible, setProviderModalVisible] = useState(false);
  const [connectedProviders, setConnectedProviders] = useState<ExtendedProviderStatus[]>([]);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [messageFeedback, setMessageFeedback] = useState<Record<string, 'up' | 'down' | null>>({});
  const [coaches, setCoaches] = useState<Coach[]>([]);
  const [pendingCoachAction, setPendingCoachAction] = useState<{ coach: Coach } | null>(null);
  const [renamePromptVisible, setRenamePromptVisible] = useState(false);
  const [renameConversationId, setRenameConversationId] = useState<string | null>(null);
  const [renameDefaultTitle, setRenameDefaultTitle] = useState('');

  // Share to social feed state
  const [shareToFeedContent, setShareToFeedContent] = useState<string | null>(null);
  const [shareToFeedVisibility, setShareToFeedVisibility] = useState<'friends_only' | 'public'>('friends_only');
  const [isSharing, setIsSharing] = useState(false);

  // Voice input hook for speech-to-text
  const {
    isListening,
    transcript,
    partialTranscript,
    error: voiceError,
    isAvailable: voiceAvailable,
    startListening,
    stopListening,
    clearTranscript,
    clearError: clearVoiceError,
  } = useVoiceInput();

  const flatListRef = useRef<FlatList>(null);
  const inputRef = useRef<TextInput>(null);
  // Track when we just created a conversation to prevent loadMessages from clearing optimistic messages
  const justCreatedConversationRef = useRef<string | null>(null);

  // Load conversations, prompts, provider status, and coaches when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      loadConversations();
      loadProviderStatus();
      loadCoaches();
    }
  }, [isAuthenticated]);

  const loadProviderStatus = async () => {
    try {
      const response = await oauthApi.getProvidersStatus();
      setConnectedProviders(response.providers || []);
    } catch (error) {
      console.error('Failed to load provider status:', error);
    }
  };

  const loadCoaches = async () => {
    try {
      const response = await coachesApi.list();
      // Sort: favorites first, then by use_count descending
      const sorted = [...response.coaches].sort((a, b) => {
        if (a.is_favorite !== b.is_favorite) {
          return a.is_favorite ? -1 : 1;
        }
        return b.use_count - a.use_count;
      });
      setCoaches(sorted);
    } catch (error) {
      console.error('Failed to load coaches:', error);
    }
  };

  // Refresh provider status when app returns from OAuth flow
  useEffect(() => {
    const subscription = AppState.addEventListener('change', (nextAppState) => {
      if (nextAppState === 'active') {
        loadProviderStatus();
      }
    });
    return () => subscription.remove();
  }, []);

  // Handle voice input transcript - replace input text with final transcript
  useEffect(() => {
    if (transcript) {
      // Replace input with transcript (don't append - causes duplication)
      setInputText(transcript);
      clearTranscript();
    }
  }, [transcript, clearTranscript]);

  // Handle voice input errors - show toast notifications
  useEffect(() => {
    if (voiceError) {
      const showVoiceErrorToast = (error: VoiceError) => {
        // Determine toast type and actions based on error type
        if (error.type === 'permission_denied') {
          Toast.show({
            type: 'voiceError',
            text1: 'Microphone Access Required',
            text2: error.message,
            visibilityTime: 5000,
            props: {
              onOpenSettings: () => {
                Linking.openSettings();
                clearVoiceError();
              },
            },
          });
        } else if (error.type === 'no_speech') {
          Toast.show({
            type: 'voiceError',
            text1: 'No Speech Detected',
            text2: error.message,
            visibilityTime: 3000,
            props: {
              onRetry: () => {
                clearVoiceError();
                startListening();
              },
            },
          });
        } else if (error.type === 'network_error') {
          Toast.show({
            type: 'voiceError',
            text1: 'Network Error',
            text2: error.message,
            visibilityTime: 4000,
            props: {
              onRetry: () => {
                clearVoiceError();
                startListening();
              },
            },
          });
        } else if (error.type === 'timeout') {
          Toast.show({
            type: 'info',
            text1: 'Voice Input Timeout',
            text2: error.message,
            visibilityTime: 3000,
          });
        } else {
          Toast.show({
            type: 'error',
            text1: 'Voice Input Error',
            text2: error.message,
            visibilityTime: 3000,
          });
        }
      };

      showVoiceErrorToast(voiceError);
      // Clear error after showing toast (for non-action toasts)
      if (voiceError.type !== 'permission_denied' && voiceError.type !== 'no_speech' && voiceError.type !== 'network_error') {
        clearVoiceError();
      }
    }
  }, [voiceError, clearVoiceError, startListening]);

  const hasConnectedProvider = (): boolean => {
    return connectedProviders.some(p => p.connected);
  };

  // Load messages when conversation changes (but not when we just created the conversation)
  useEffect(() => {
    if (currentConversation) {
      // Skip loading if we just created this conversation - we already have optimistic messages
      if (justCreatedConversationRef.current === currentConversation.id) {
        justCreatedConversationRef.current = null;
        return;
      }
      loadMessages(currentConversation.id);
    } else {
      setMessages([]);
    }
    // loadMessages is intentionally omitted - including it would cause infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentConversation]);

  // Handle explicit "new chat" navigation from drawer (conversationId becomes undefined)
  useEffect(() => {
    const conversationId = route.params?.conversationId;
    if (conversationId === undefined && currentConversation !== null) {
      // User explicitly navigated to new chat - clear state
      setCurrentConversation(null);
      setMessages([]);
    }
    // Only depend on route params - this should only run when user navigates
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.params?.conversationId]);

  // Handle conversation selection from drawer navigation
  useEffect(() => {
    const conversationId = route.params?.conversationId;
    if (conversationId && conversations.length > 0) {
      // Find and select the conversation
      const conversation = conversations.find(c => c.id === conversationId);
      // Update if ID is different OR if current has no title but loaded one does
      const shouldUpdate = conversation && (
        conversation.id !== currentConversation?.id ||
        (!currentConversation?.title && conversation.title)
      );
      if (shouldUpdate) {
        setCurrentConversation(conversation);
      }
    }
    // currentConversation intentionally omitted - including it would cause infinite loops
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.params?.conversationId, conversations]);

  const loadConversations = async () => {
    try {
      setIsLoading(true);
      const response = await chatApi.getConversations();
      // Deduplicate by ID to prevent duplicate key warnings
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv: { id: string }) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      // Sort by updated_at descending (newest first)
      const sorted = deduplicated.sort((a: { updated_at?: string }, b: { updated_at?: string }) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : 0;
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : 0;
        return dateB - dateA;
      });
      setConversations(sorted);
    } catch (error) {
      console.error('Failed to load conversations:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const loadMessages = async (conversationId: string) => {
    try {
      const response = await chatApi.getConversationMessages(conversationId);
      setMessages(response.messages);
      setTimeout(() => scrollToBottom(), 100);
    } catch (error) {
      console.error('Failed to load messages:', error);
    }
  };

  const scrollToBottom = () => {
    if (flatListRef.current && messages.length > 0) {
      flatListRef.current.scrollToEnd({ animated: true });
    }
  };

  const handleNewChat = () => {
    // Clear state to show welcome screen with prompts
    setCurrentConversation(null);
    setMessages([]);
    setIsSending(false);
  };

  const handleDeleteConversation = async (conversationId: string) => {
    try {
      await chatApi.deleteConversation(conversationId);
      setConversations(prev => prev.filter(c => c.id !== conversationId));
      if (currentConversation?.id === conversationId) {
        setCurrentConversation(null);
      }
    } catch {
      Alert.alert('Error', 'Failed to delete conversation');
    }
  };

  const handleRenameConversation = (conversationId: string, currentTitle: string) => {
    setRenameConversationId(conversationId);
    setRenameDefaultTitle(currentTitle);
    setRenamePromptVisible(true);
  };

  const handleRenameSubmit = async (newTitle: string) => {
    setRenamePromptVisible(false);
    if (!renameConversationId) return;

    try {
      const updated = await chatApi.updateConversation(renameConversationId, {
        title: newTitle,
      });
      // Update conversation and move to top (most recently updated)
      setConversations(prev => {
        const updatedConv = prev.find(c => c.id === renameConversationId);
        if (!updatedConv) return prev;
        const others = prev.filter(c => c.id !== renameConversationId);
        return [
          { ...updatedConv, title: updated.title, updated_at: updated.updated_at },
          ...others,
        ];
      });
      // Always update currentConversation if IDs match
      setCurrentConversation(prev => {
        if (prev?.id === renameConversationId) {
          return { ...prev, title: updated.title, updated_at: updated.updated_at };
        }
        return prev;
      });
    } catch (error) {
      console.error('Failed to rename conversation:', error);
      Alert.alert('Error', 'Failed to rename conversation');
    } finally {
      setRenameConversationId(null);
      setRenameDefaultTitle('');
    }
  };

  const handleRenameCancel = () => {
    setRenamePromptVisible(false);
    setRenameConversationId(null);
    setRenameDefaultTitle('');
  };

  const showTitleActionMenu = () => {
    if (!currentConversation) return;
    setActionMenuVisible(true);
  };

  const handleMenuRename = () => {
    setActionMenuVisible(false);
    if (currentConversation) {
      // Use fallback if title is undefined (defensive fix)
      const title = currentConversation.title || 'New Chat';
      handleRenameConversation(currentConversation.id, title);
    }
  };

  const handleMenuDelete = () => {
    setActionMenuVisible(false);
    if (!currentConversation) return;

    Alert.alert(
      'Delete Conversation',
      `Are you sure you want to delete "${currentConversation.title || 'this conversation'}"?`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: () => handleDeleteConversation(currentConversation.id),
        },
      ]
    );
  };

  const closeActionMenu = () => {
    setActionMenuVisible(false);
  };

  const handleCopyMessage = async (content: string) => {
    try {
      await Clipboard.setStringAsync(content);
      Alert.alert('Copied', 'Message copied to clipboard');
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  const handleShareMessage = async (content: string) => {
    try {
      await Share.share({ message: content });
    } catch (error) {
      console.error('Failed to share:', error);
    }
  };

  const handleShareToFeed = async () => {
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
  };

  const handleThumbsUp = (messageId: string) => {
    setMessageFeedback(prev => ({
      ...prev,
      [messageId]: prev[messageId] === 'up' ? null : 'up',
    }));
  };

  const handleThumbsDown = (messageId: string) => {
    setMessageFeedback(prev => ({
      ...prev,
      [messageId]: prev[messageId] === 'down' ? null : 'down',
    }));
  };

  const handleRetryMessage = async (messageId: string) => {
    // Find the assistant message and the preceding user message
    const messageIndex = messages.findIndex(m => m.id === messageId);
    if (messageIndex > 0) {
      const userMessage = messages[messageIndex - 1];
      if (userMessage.role === 'user') {
        // Remove the assistant message
        setMessages(prev => prev.filter(m => m.id !== messageId));
        // Resend the user's prompt
        await sendPromptMessage(userMessage.content);
      }
    }
  };

  // Toggle voice input recording with haptic feedback
  const handleVoicePress = async () => {
    // Haptic feedback on press
    await Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);

    if (isListening) {
      await stopListening();
    } else {
      // Clear input before starting voice to avoid mixing with previous text
      setInputText('');
      await startListening();
    }
  };

  const handleSendMessage = async () => {
    if (!inputText.trim() || isSending) return;

    const messageText = inputText.trim();
    setInputText('');
    setIsSending(true);

    // Create conversation if needed
    let conversationId = currentConversation?.id;
    if (!conversationId) {
      try {
        const conversation = await chatApi.createConversation({
          title: messageText.slice(0, 50),
        });
        if (!conversation || !conversation.id) {
          throw new Error('Invalid conversation response');
        }
        setConversations(prev => [conversation, ...prev]);
        // Mark this as just-created to prevent useEffect from clearing optimistic messages
        justCreatedConversationRef.current = conversation.id;
        setCurrentConversation(conversation);
        conversationId = conversation.id;
      } catch (error) {
        console.error('Failed to create conversation:', error);
        Alert.alert('Error', 'Failed to create conversation');
        setIsSending(false);
        return;
      }
    }

    // Add user message optimistically
    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      role: 'user',
      content: messageText,
      created_at: new Date().toISOString(),
    };
    setMessages(prev => [...prev, userMessage]);
    scrollToBottom();

    try {
      const response = await chatApi.sendMessage(conversationId, messageText);
      // Replace optimistic user message and add assistant response with metadata
      // Defensive: only add messages if they have valid IDs
      setMessages(prev => {
        const filtered = prev.filter(m => m.id !== userMessage.id);
        const newMessages: Message[] = [];
        if (response.user_message?.id) {
          newMessages.push(response.user_message);
        }
        if (response.assistant_message?.id) {
          newMessages.push({
            ...response.assistant_message,
            model: response.model,
            execution_time_ms: response.execution_time_ms,
          });
        }
        return [...filtered, ...newMessages];
      });
      scrollToBottom();
    } catch (error) {
      console.error('Failed to send message:', error);
      // Keep user message and add error response inline (no popup)
      const errorMessage = error instanceof Error ? error.message : 'Failed to send message';
      const errorResponse: Message = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `⚠️ ${errorMessage}\n\nPlease try again.`,
        created_at: new Date().toISOString(),
        isError: true,
      };
      setMessages(prev => {
        // Replace temp message with persisted version and add error
        const updated = prev.map(m =>
          m.id === userMessage.id ? { ...m, id: `user-${Date.now()}` } : m
        );
        return [...updated, errorResponse];
      });
      scrollToBottom();
    } finally {
      setIsSending(false);
    }
  };

  const handleCoachSelect = async (coach: Coach) => {
    if (isSending) return;

    // Check if any provider is connected
    if (!hasConnectedProvider()) {
      // Store pending action and show provider modal
      setPendingCoachAction({ coach });
      setProviderModalVisible(true);
      return;
    }

    await startCoachConversation(coach);
  };

  const startCoachConversation = async (coach: Coach) => {
    try {
      setIsSending(true);

      // Record usage (fire-and-forget)
      coachesApi.recordUsage(coach.id);

      // Create a new conversation with the coach's system prompt
      const conversation = await chatApi.createConversation({
        title: `Chat with ${coach.title}`,
        system_prompt: coach.system_prompt,
      });

      if (!conversation || !conversation.id) {
        throw new Error('Invalid conversation response');
      }

      setConversations(prev => [conversation, ...prev]);
      justCreatedConversationRef.current = conversation.id;
      setCurrentConversation(conversation);

      // Auto-send initial message with coach description
      const initialMessage = coach.description || `Let's get started with ${coach.title}!`;

      // Add user message optimistically
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        role: 'user',
        content: initialMessage,
        created_at: new Date().toISOString(),
      };
      setMessages([userMessage]);

      // Send message to API
      const response = await chatApi.sendMessage(conversation.id, initialMessage);

      // Update with actual messages from server
      setMessages(prev => {
        const filtered = prev.filter(m => m.id !== userMessage.id);
        const newMessages: Message[] = [];
        if (response.user_message?.id) {
          newMessages.push(response.user_message);
        }
        if (response.assistant_message?.id) {
          newMessages.push({
            ...response.assistant_message,
            model: response.model,
            execution_time_ms: response.execution_time_ms,
          });
        }
        return [...filtered, ...newMessages];
      });
    } catch (error) {
      console.error('Failed to start coach conversation:', error);
      Alert.alert('Error', 'Failed to start conversation with coach');
    } finally {
      setIsSending(false);
    }
  };

  const sendPromptMessage = async (prompt: string) => {
    setIsSending(true);

    // Create conversation if needed
    let conversationId = currentConversation?.id;
    if (!conversationId) {
      try {
        const conversation = await chatApi.createConversation({
          title: prompt.slice(0, 50),
        });
        if (!conversation || !conversation.id) {
          throw new Error('Invalid conversation response');
        }
        setConversations(prev => [conversation, ...prev]);
        // Mark this as just-created to prevent useEffect from clearing optimistic messages
        justCreatedConversationRef.current = conversation.id;
        setCurrentConversation(conversation);
        conversationId = conversation.id;
      } catch (error) {
        console.error('Failed to create conversation:', error);
        Alert.alert('Error', 'Failed to create conversation');
        setIsSending(false);
        return;
      }
    }

    // Add user message optimistically
    const userMessage: Message = {
      id: `temp-${Date.now()}`,
      role: 'user',
      content: prompt,
      created_at: new Date().toISOString(),
    };
    setMessages(prev => [...prev, userMessage]);
    scrollToBottom();

    try {
      const response = await chatApi.sendMessage(conversationId, prompt);
      // Replace optimistic message with server's message and add assistant response with metadata
      // Defensive: only update/add messages if they have valid IDs
      setMessages(prev => {
        let updated = prev.map(m => {
          if (m.id === userMessage.id && response.user_message?.id) {
            // Use server's message ID but keep our original prompt content
            return { ...response.user_message, content: prompt };
          }
          return m;
        });
        if (response.assistant_message?.id) {
          updated = updated.concat([{
            ...response.assistant_message,
            model: response.model,
            execution_time_ms: response.execution_time_ms,
          }]);
        }
        return updated;
      });
      scrollToBottom();
    } catch (error) {
      console.error('Failed to send message:', error);
      // Keep user message and add error response inline (no popup)
      const errorMessage = error instanceof Error ? error.message : 'Failed to send message';
      const errorResponse: Message = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `⚠️ ${errorMessage}\n\nPlease try again.`,
        created_at: new Date().toISOString(),
        isError: true,
      };
      setMessages(prev => {
        // Replace temp message with persisted version and add error
        const updated = prev.map(m =>
          m.id === userMessage.id ? { ...m, id: `user-${Date.now()}` } : m
        );
        return [...updated, errorResponse];
      });
      scrollToBottom();
    } finally {
      setIsSending(false);
    }
  };

  const handleConnectProvider = async (provider: string) => {
    setProviderModalVisible(false);
    try {
      // Create return URL for deep linking back to the app after OAuth
      // Uses custom scheme (pierre://) for consistent behavior in dev and prod
      const returnUrl = getOAuthCallbackUrl();

      // Call the mobile OAuth init endpoint which returns the authorization URL
      const oauthResponse = await oauthApi.initMobileOAuth(provider, returnUrl);

      // Open OAuth in an in-app browser (ASWebAuthenticationSession on iOS)
      // The returnUrl is watched for redirects to close the browser automatically
      const result = await WebBrowser.openAuthSessionAsync(
        oauthResponse.authorization_url,
        returnUrl
      );

      if (result.type === 'success' && result.url) {
        // Parse the return URL to check for success/error
        const parsedUrl = Linking.parse(result.url);
        const success = parsedUrl.queryParams?.success === 'true';
        const error = parsedUrl.queryParams?.error as string | undefined;

        if (success) {
          // OAuth completed successfully - refresh connection status
          await loadProviderStatus();
          // Start pending coach conversation now that provider is connected
          if (pendingCoachAction) {
            await startCoachConversation(pendingCoachAction.coach);
            setPendingCoachAction(null);
          } else if (pendingPrompt) {
            await sendPromptMessage(pendingPrompt);
            setPendingPrompt(null);
          }
        } else if (error) {
          console.error('OAuth error from server:', error);
          Alert.alert('Connection Failed', `Failed to connect: ${error}`);
        } else {
          // No explicit success/error - refresh status and check
          await loadProviderStatus();
          Alert.alert('Connection Complete', `${provider} connection flow completed.`);
        }
      } else if (result.type === 'cancel') {
        // User cancelled - keep pending actions so they can try again
        console.log('OAuth cancelled by user');
      }
    } catch (error) {
      console.error('Failed to start OAuth:', error);
      Alert.alert('Error', 'Failed to connect provider. Please try again.');
    }
  };

  // Helper to detect OAuth authorization URLs using secure hostname validation
  const isOAuthUrl = (url: string): { isOAuth: boolean; provider: string | null } => {
    try {
      const parsedUrl = new URL(url);
      const hostname = parsedUrl.hostname.toLowerCase();

      if (hostname === 'www.strava.com' || hostname === 'strava.com') {
        if (parsedUrl.pathname.includes('/oauth/authorize')) {
          return { isOAuth: true, provider: 'Strava' };
        }
      }
      if (hostname === 'www.fitbit.com' || hostname === 'fitbit.com') {
        if (parsedUrl.pathname.includes('/oauth2/authorize')) {
          return { isOAuth: true, provider: 'Fitbit' };
        }
      }
      if (hostname.endsWith('.garmin.com') || hostname === 'garmin.com') {
        if (url.includes('oauth')) {
          return { isOAuth: true, provider: 'Garmin' };
        }
      }
      return { isOAuth: false, provider: null };
    } catch {
      // Invalid URL - not an OAuth URL
      return { isOAuth: false, provider: null };
    }
  };

  // Helper to open URLs in browser
  const handleOpenUrl = async (url: string) => {
    try {
      const { isOAuth } = isOAuthUrl(url);
      if (isOAuth) {
        // Use in-app browser for OAuth
        await WebBrowser.openBrowserAsync(url);
      } else {
        // Use system browser for regular links
        await Linking.openURL(url);
      }
    } catch (error) {
      console.error('Failed to open URL:', error);
      Alert.alert('Error', 'Failed to open link');
    }
  };

  // Markdown styles for assistant messages
  const markdownStyles = {
    body: {
      color: colors.text.primary,
      fontSize: fontSize.md,
      lineHeight: fontSize.md * 1.5,
    },
    heading1: {
      color: colors.text.primary,
      fontSize: fontSize.xl,
      fontWeight: '700' as const,
      marginTop: spacing.md,
      marginBottom: spacing.sm,
    },
    heading2: {
      color: colors.text.primary,
      fontSize: fontSize.lg,
      fontWeight: '600' as const,
      marginTop: spacing.sm,
      marginBottom: spacing.xs,
    },
    heading3: {
      color: colors.text.primary,
      fontSize: fontSize.md,
      fontWeight: '600' as const,
      marginTop: spacing.xs,
      marginBottom: spacing.xs,
    },
    strong: {
      color: colors.text.primary,
      fontWeight: '700' as const,
    },
    em: {
      color: colors.text.secondary,
      fontStyle: 'italic' as const,
    },
    bullet_list: {
      marginLeft: spacing.sm,
    },
    ordered_list: {
      marginLeft: spacing.sm,
    },
    list_item: {
      marginBottom: spacing.xs,
    },
    code_inline: {
      backgroundColor: colors.background.tertiary,
      color: colors.primary[400],
      paddingHorizontal: 4,
      borderRadius: 4,
      fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
      fontSize: fontSize.sm,
    },
    fence: {
      backgroundColor: colors.background.tertiary,
      borderRadius: borderRadius.sm,
      padding: spacing.sm,
      marginVertical: spacing.xs,
    },
    code_block: {
      fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
      fontSize: fontSize.sm,
      color: colors.text.primary,
    },
    link: {
      color: colors.primary[400],
      textDecorationLine: 'underline' as const,
    },
    hr: {
      backgroundColor: colors.border.default,
      height: 1,
      marginVertical: spacing.sm,
    },
  };

  // Render message content with markdown support for assistant and clickable links
  const renderMessageContent = (content: string, isUser: boolean) => {
    // For user messages, render plain text
    if (isUser) {
      return (
        <Text className="text-base text-text-primary leading-6">
          {content}
        </Text>
      );
    }

    // For assistant messages, check for OAuth URLs first
    const urlRegex = /https?:\/\/[^\s<>"\]]+/gi;
    const oauthUrls = content.match(urlRegex)?.filter(url => {
      const { isOAuth } = isOAuthUrl(url);
      return isOAuth;
    }) || [];

    // If there are OAuth URLs, render them as buttons above the markdown content
    if (oauthUrls.length > 0) {
      // Remove OAuth URLs from content for cleaner markdown rendering
      let cleanContent = content;
      oauthUrls.forEach(url => {
        const escapedUrl = url.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        // Remove markdown image syntax: ![alt](url)
        cleanContent = cleanContent.replace(new RegExp(`!\\[([^\\]]*)\\]\\(${escapedUrl}\\)`, 'g'), '');
        // Remove markdown link syntax: [text](url)
        cleanContent = cleanContent.replace(new RegExp(`\\[([^\\]]*)\\]\\(${escapedUrl}\\)`, 'g'), '');
        // Remove plain URL
        cleanContent = cleanContent.replace(url, '');
      });

      return (
        <View className="flex-row flex-wrap items-center">
          {oauthUrls.map((url, index) => {
            const { provider } = isOAuthUrl(url);
            return (
              <TouchableOpacity
                key={`oauth-${index}`}
                className="px-4 py-2 rounded-lg my-1 self-start"
                style={{ backgroundColor: colors.providers.strava }}
                onPress={() => handleOpenUrl(url)}
              >
                <Text className="text-base font-semibold text-text-primary">
                  Connect to {provider}
                </Text>
              </TouchableOpacity>
            );
          })}
          {cleanContent.trim() && (
            <Markdown style={markdownStyles} onLinkPress={(url) => { handleOpenUrl(url); return false; }}>
              {cleanContent.trim()}
            </Markdown>
          )}
        </View>
      );
    }

    // Render markdown for assistant messages without OAuth URLs
    return (
      <Markdown style={markdownStyles} onLinkPress={(url) => { handleOpenUrl(url); return false; }}>
        {content}
      </Markdown>
    );
  };

  const renderMessage = ({ item }: { item: Message }) => {
    // Defensive: skip rendering if item is invalid
    if (!item?.id) return null;

    const isUser = item.role === 'user';
    const isError = item.isError === true;

    // Format timestamp
    const timestamp = item.created_at ? new Date(item.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';

    return (
      <View className={`mb-4 ${isUser ? 'items-end' : ''}`}>
        {/* Timestamp - muted zinc-500 per Stitch spec */}
        <Text className="text-xs text-zinc-500 mb-1 px-1">{timestamp}</Text>
        <View
          className={`flex-row max-w-[85%] rounded-2xl p-4 ${
            isUser
              ? 'rounded-br-[4px]'
              : 'rounded-bl-[4px]'
          } ${isError ? 'border border-error' : ''}`}
          style={[
            // User bubbles: violet (#8B5CF6) per Stitch spec
            isUser ? { backgroundColor: colors.pierre.violet } : undefined,
            // AI bubbles: glassmorphic dark slate per Stitch spec
            !isUser && !isError ? {
              backgroundColor: 'rgba(30, 30, 46, 0.9)',
              borderWidth: 1,
              borderColor: 'rgba(255, 255, 255, 0.1)',
            } : undefined,
            isError ? { backgroundColor: 'rgba(239, 68, 68, 0.15)' } : undefined,
          ]}
        >
          {!isUser && (
            <View className="w-8 h-8 rounded-full mr-2 overflow-hidden">
              <Image
                source={require('../../../assets/pierre-logo.png')}
                className="w-8 h-8"
                resizeMode="cover"
              />
            </View>
          )}
          <View className="flex-1">
            {renderMessageContent(item.content, isUser)}
          </View>
        </View>
        {/* Action icons and metadata for assistant messages */}
        {!isUser && (
          <View className="flex-row mt-1 gap-4">
            {isError ? (
              /* For error messages, show only Retry button */
              <TouchableOpacity
                className="flex-row items-center bg-background-tertiary px-2 py-1 rounded gap-1"
                onPress={() => handleRetryMessage(item.id)}
              >
                <Ionicons name="refresh-outline" size={14} color={colors.text.primary} />
                <Text className="text-xs text-text-primary font-medium">Retry</Text>
              </TouchableOpacity>
            ) : (
              /* Normal assistant message actions */
              <>
                {/* Copy */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => handleCopyMessage(item.content)}
                >
                  <Ionicons name="copy-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Share (system share sheet) */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => handleShareMessage(item.content)}
                >
                  <Ionicons name="arrow-redo-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Share to Social Feed */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => setShareToFeedContent(item.content)}
                >
                  <Ionicons name="people-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Thumbs Up */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => handleThumbsUp(item.id)}
                >
                  <Ionicons
                    name={messageFeedback[item.id] === 'up' ? 'thumbs-up' : 'thumbs-up-outline'}
                    size={14}
                    color={messageFeedback[item.id] === 'up' ? colors.pierre.violet : colors.text.tertiary}
                  />
                </TouchableOpacity>
                {/* Thumbs Down */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => handleThumbsDown(item.id)}
                >
                  <Ionicons
                    name={messageFeedback[item.id] === 'down' ? 'thumbs-down' : 'thumbs-down-outline'}
                    size={14}
                    color={messageFeedback[item.id] === 'down' ? colors.error : colors.text.tertiary}
                  />
                </TouchableOpacity>
                {/* Retry */}
                <TouchableOpacity
                  className="p-0.5"
                  onPress={() => handleRetryMessage(item.id)}
                >
                  <Ionicons name="refresh-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Model and response time - to the right of icons */}
                {item.model && (
                  <Text className="text-xs text-text-tertiary ml-2">
                    {item.model}{item.execution_time_ms ? ` · ${(item.execution_time_ms / 1000).toFixed(1)}s` : ''}
                  </Text>
                )}
              </>
            )}
          </View>
        )}
      </View>
    );
  };

  const renderThinkingIndicator = () => (
    <View className="mb-4">
      <View
        className="flex-row max-w-[85%] rounded-2xl rounded-bl-[4px] p-4"
        style={{
          backgroundColor: 'rgba(30, 30, 46, 0.9)',
          borderWidth: 1,
          borderColor: 'rgba(255, 255, 255, 0.1)',
        }}
      >
        <View className="w-8 h-8 rounded-full mr-3 overflow-hidden">
          <Image
            source={require('../../../assets/pierre-logo.png')}
            className="w-8 h-8"
            resizeMode="cover"
          />
        </View>
        {/* Pulsing dots typing indicator per Stitch spec */}
        <View className="flex-row items-center gap-1">
          <View className="w-2 h-2 rounded-full bg-pierre-violet opacity-60" />
          <View className="w-2 h-2 rounded-full bg-pierre-violet opacity-80" />
          <View className="w-2 h-2 rounded-full bg-pierre-violet" />
        </View>
      </View>
    </View>
  );

  // Check if current conversation is a coach conversation (has system_prompt)
  const isCoachConversation = Boolean(currentConversation?.system_prompt);

  // Render a single coach card for the grid (matching web design)
  const renderCoachGridCard = (coach: Coach) => (
    <TouchableOpacity
      key={coach.id}
      className="bg-background-secondary rounded-xl p-4 w-[48%] border border-border-subtle mb-2"
      onPress={() => handleCoachSelect(coach)}
      activeOpacity={0.7}
    >
      {/* Header row: Title (full width, wraps to 2 lines) + Category icon */}
      <View className="flex-row justify-between items-start mb-1 gap-2">
        <Text className="flex-1 text-sm font-semibold text-text-primary leading-[18px]" numberOfLines={2}>
          {coach.title}
        </Text>
        <View
          className="w-7 h-7 rounded items-center justify-center"
          style={{ backgroundColor: COACH_CATEGORY_BADGE_BG[coach.category] }}
        >
          <Text className="text-sm">
            {COACH_CATEGORY_ICONS[coach.category]}
          </Text>
        </View>
      </View>
      {/* Description */}
      {coach.description && (
        <Text className="text-xs text-text-secondary leading-4 mb-1" numberOfLines={2}>
          {coach.description}
        </Text>
      )}
      {/* Footer: Badges + Use count */}
      <View className="flex-row items-center gap-2 mt-1">
        {coach.is_system && (
          <View className="px-2 py-0.5 rounded" style={{ backgroundColor: 'rgba(124, 58, 237, 0.15)' }}>
            <Text className="text-xs font-medium" style={{ color: '#7C3AED' }}>System</Text>
          </View>
        )}
        {coach.is_favorite && (
          <View className="px-1 py-0.5 rounded" style={{ backgroundColor: 'rgba(245, 158, 11, 0.15)' }}>
            <Text className="text-xs" style={{ color: '#F59E0B' }}>★</Text>
          </View>
        )}
        <View className="flex-1" />
        {coach.use_count > 0 && (
          <Text className="text-xs text-text-tertiary">{coach.use_count}×</Text>
        )}
      </View>
    </TouchableOpacity>
  );

  const renderEmptyChat = () => (
    <ScrollView
      className="flex-1"
      contentContainerStyle={{ flexGrow: 1, alignItems: 'center', justifyContent: 'flex-start', paddingHorizontal: spacing.xs, paddingVertical: spacing.md, paddingBottom: 100 }}
      showsVerticalScrollIndicator={false}
      keyboardShouldPersistTaps="handled"
    >
      {/* Coach Grid - only show when NOT in a coach conversation */}
      {!isCoachConversation && coaches.length > 0 && (
        <View className="w-full px-1">
          <Text className="text-lg font-semibold text-text-primary mb-4">🎯 Your Coaches</Text>
          <View className="flex-row flex-wrap justify-between gap-2">
            {coaches.map((coach) => renderCoachGridCard(coach))}
          </View>
        </View>
      )}

      {/* Empty state when no coaches */}
      {!isCoachConversation && coaches.length === 0 && (
        <View className="flex-1 items-center justify-center px-8 py-12">
          <Text className="text-lg font-semibold text-text-primary mb-2">No coaches yet</Text>
          <Text className="text-base text-text-tertiary text-center">
            Create your first coach to customize how Pierre helps you.
          </Text>
        </View>
      )}

      {/* Coach conversation starter */}
      {isCoachConversation && (
        <View className="w-full items-center px-4 mb-6">
          <Text className="text-base text-text-secondary text-center leading-6">
            Your coach is ready. Start the conversation by typing a message below.
          </Text>
        </View>
      )}

    </ScrollView>
  );

  return (
    <View className="flex-1 bg-background-primary" testID="chat-screen">
      <KeyboardAvoidingView
        className="flex-1"
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
        keyboardVerticalOffset={Platform.OS === 'ios' ? 0 : 0}
      >
        {/* Header with safe area inset for status bar - Stitch spec */}
        <View
          className="flex-row items-center px-4 py-2 border-b border-border-subtle"
          style={{ paddingTop: insets.top + spacing.sm }}
        >
          {/* Back arrow or history button */}
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center"
            onPress={() => currentConversation ? handleNewChat() : navigation.navigate('Conversations')}
            testID="history-button"
          >
            <Ionicons
              name={currentConversation ? 'arrow-back' : 'time-outline'}
              size={24}
              color={colors.text.primary}
            />
          </TouchableOpacity>

          {/* Coach avatar with status dot when in conversation */}
          {currentConversation && (
            <View className="relative mr-2">
              <View className="w-10 h-10 rounded-full overflow-hidden bg-pierre-slate items-center justify-center">
                <Image
                  source={require('../../../assets/pierre-logo.png')}
                  className="w-10 h-10"
                  resizeMode="cover"
                />
              </View>
              {/* Pulsing green status dot per Stitch spec */}
              <View className="absolute bottom-0 right-0 w-3 h-3 rounded-full bg-pierre-activity border-2 border-background-primary" />
            </View>
          )}

          <TouchableOpacity
            className={`flex-1 flex-row items-center ${currentConversation ? '' : 'justify-center'} mx-2 ${actionMenuVisible ? 'opacity-0' : ''}`}
            onPress={showTitleActionMenu}
            disabled={!currentConversation}
            testID="chat-title-button"
          >
            <Text className="text-lg font-semibold text-text-primary" numberOfLines={1} testID="chat-title">
              {currentConversation?.title || 'New Chat'}
            </Text>
            {currentConversation && (
              <Text className="text-[10px] ml-1 text-text-tertiary">▼</Text>
            )}
          </TouchableOpacity>
          <TouchableOpacity
            className="w-10 h-10 items-center justify-center bg-background-tertiary rounded-lg"
            onPress={handleNewChat}
            testID="new-chat-button"
          >
            <Text className="text-2xl text-text-primary font-light">+</Text>
          </TouchableOpacity>
        </View>

        {/* Messages or Empty State */}
        {isLoading ? (
          <View className="flex-1 items-center justify-center">
            <ActivityIndicator size="large" color={colors.primary[500]} />
          </View>
        ) : ((messages?.length ?? 0) === 0 && !isSending) ? (
          renderEmptyChat()
        ) : (
          <FlatList
            ref={flatListRef}
            data={messages ?? []}
            renderItem={renderMessage}
            keyExtractor={(item, index) => item?.id ? `${item.id}-${index}` : `fallback-${index}`}
            contentContainerStyle={{ paddingHorizontal: spacing.md, paddingVertical: spacing.md, paddingBottom: 80 }}
            showsVerticalScrollIndicator={false}
            onContentSizeChange={scrollToBottom}
            ListFooterComponent={isSending ? renderThinkingIndicator : null}
          />
        )}

        {/* Input Area - liquid style with violet accent, transparent background */}
        <View className="absolute bottom-0 left-0 right-0 px-4 py-2">
          <View
            className="flex-row items-center rounded-full px-3 min-h-[36px] max-h-[100px]"
            style={{ backgroundColor: 'rgba(30, 27, 45, 0.95)', borderWidth: 1, borderColor: 'rgba(139, 92, 246, 0.4)' }}
          >
            {/* Paperclip icon per Stitch spec */}
            <TouchableOpacity className="w-8 h-8 items-center justify-center mr-1">
              <Ionicons name="attach-outline" size={22} color={colors.text.tertiary} />
            </TouchableOpacity>
            <TextInput
              ref={inputRef}
              className="flex-1 text-base text-text-primary py-2 max-h-[100px]"
              placeholder={isListening ? 'Listening...' : 'Message Pierre...'}
              placeholderTextColor={isListening ? colors.error : colors.text.tertiary}
              value={isListening ? partialTranscript : inputText}
              onChangeText={setInputText}
              multiline
              maxLength={4000}
              returnKeyType="default"
              editable={!isListening}
              testID="message-input"
            />
            <VoiceButton
              isListening={isListening}
              isAvailable={voiceAvailable}
              onPress={handleVoicePress}
              disabled={isSending}
              size="sm"
              testID="voice-input-button"
            />
            {/* Violet send button per Stitch spec */}
            <TouchableOpacity
              className={`w-9 h-9 rounded-full items-center justify-center ml-2 ${
                !inputText.trim() || isSending || isListening ? 'bg-background-tertiary' : ''
              }`}
              style={inputText.trim() && !isSending && !isListening ? { backgroundColor: colors.pierre.violet } : undefined}
              onPress={handleSendMessage}
              disabled={!inputText.trim() || isSending || isListening}
              testID="send-button"
            >
              {isSending ? (
                <ActivityIndicator size="small" color={colors.text.primary} />
              ) : (
                <Ionicons name="arrow-up" size={20} color={colors.text.primary} />
              )}
            </TouchableOpacity>
          </View>
          {isListening && (
            <View className="pt-1 items-center">
              <Text className="text-xs text-error">Tap mic to stop recording</Text>
            </View>
          )}
        </View>

        {/* Conversation Action Menu Modal - Claude-style popover */}
        <Modal
          visible={actionMenuVisible}
          animationType="fade"
          transparent
          onRequestClose={closeActionMenu}
        >
          <TouchableOpacity
            className="flex-1 bg-black/30"
            activeOpacity={1}
            onPress={closeActionMenu}
          >
            <View style={popoverContainerStyle}>
              <TouchableOpacity
                className="flex-row items-center px-4 py-3 opacity-40"
                disabled
              >
                <Ionicons name="star-outline" size={20} color={colors.text.tertiary} style={{ marginRight: spacing.md, width: 24 }} />
                <Text className="text-base text-text-tertiary">Add to favorites</Text>
              </TouchableOpacity>

              <TouchableOpacity
                className="flex-row items-center px-4 py-3"
                onPress={handleMenuRename}
              >
                <Ionicons name="pencil-outline" size={20} color={colors.text.primary} style={{ marginRight: spacing.md, width: 24 }} />
                <Text className="text-base text-text-primary">Rename</Text>
              </TouchableOpacity>

              <TouchableOpacity
                className="flex-row items-center px-4 py-3"
                onPress={handleMenuDelete}
              >
                <Ionicons name="trash-outline" size={20} color={colors.error} style={{ marginRight: spacing.md, width: 24 }} />
                <Text className="text-base text-error">Delete</Text>
              </TouchableOpacity>
            </View>
          </TouchableOpacity>
        </Modal>

        {/* Provider Selection Modal */}
        <Modal
          visible={providerModalVisible}
          animationType="fade"
          transparent
          onRequestClose={() => {
            setProviderModalVisible(false);
            setPendingPrompt(null);
          }}
        >
          <TouchableOpacity
            className="flex-1 bg-black/50 justify-center items-center"
            activeOpacity={1}
            onPress={() => {
              setProviderModalVisible(false);
              setPendingPrompt(null);
            }}
          >
            <View style={providerModalContainerStyle}>
              <Text className="text-lg font-semibold text-text-primary text-center mb-1">Connect a Provider</Text>
              <Text className="text-sm text-text-secondary text-center mb-6">
                To analyze your fitness data, please connect a provider first.
              </Text>

              {connectedProviders.map((provider) => {
                const icon = PROVIDER_ICONS[provider.provider] || '🔗';
                const isConnected = provider.connected;
                const requiresOAuth = provider.requires_oauth;
                const displayName = provider.display_name || provider.provider;

                return (
                  <TouchableOpacity
                    key={provider.provider}
                    className={`flex-row items-center bg-background-secondary rounded-lg p-4 mb-2 border ${
                      isConnected ? 'border-accent-primary' : 'border-border-default'
                    }`}
                    onPress={() => {
                      if (isConnected) {
                        setProviderModalVisible(false);
                        if (pendingPrompt) {
                          const prompt = pendingPrompt;
                          setPendingPrompt(null);
                          sendPromptMessage(prompt);
                        }
                        if (pendingCoachAction) {
                          const coachAction = pendingCoachAction;
                          setPendingCoachAction(null);
                          startCoachConversation(coachAction.coach);
                        }
                      } else if (requiresOAuth) {
                        handleConnectProvider(provider.provider);
                      }
                    }}
                    disabled={!isConnected && !requiresOAuth}
                  >
                    <Text className="text-2xl mr-4">{icon}</Text>
                    <View className="flex-1">
                      <Text className="text-base text-text-primary font-medium">
                        {isConnected ? displayName : `Connect ${displayName}`}
                      </Text>
                      {isConnected && (
                        <Text className="text-xs text-accent-primary">Connected ✓</Text>
                      )}
                    </View>
                  </TouchableOpacity>
                );
              })}

              <TouchableOpacity
                className="items-center p-4 mt-1"
                onPress={() => {
                  setProviderModalVisible(false);
                  setPendingPrompt(null);
                  setPendingCoachAction(null);
                }}
              >
                <Text className="text-base text-text-tertiary">Cancel</Text>
              </TouchableOpacity>
            </View>
          </TouchableOpacity>
        </Modal>

        {/* Rename Conversation Prompt Dialog */}
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

        {/* Share to Social Feed Modal */}
        <Modal
          visible={shareToFeedContent !== null}
          animationType="fade"
          transparent
          onRequestClose={() => {
            setShareToFeedContent(null);
            setShareToFeedVisibility('friends_only');
          }}
        >
          <TouchableOpacity
            className="flex-1 bg-black/50 justify-center items-center"
            activeOpacity={1}
            onPress={() => {
              setShareToFeedContent(null);
              setShareToFeedVisibility('friends_only');
            }}
          >
            <View style={providerModalContainerStyle}>
              <Text className="text-lg font-semibold text-text-primary text-center mb-1">
                Share to Social Feed
              </Text>
              <Text className="text-sm text-text-secondary text-center mb-4">
                Share this insight with your friends
              </Text>

              {/* Message Preview */}
              <View className="bg-background-secondary rounded-lg p-3 mb-4 max-h-32">
                <ScrollView showsVerticalScrollIndicator={false}>
                  <Text className="text-sm text-text-primary" numberOfLines={5}>
                    {shareToFeedContent}
                  </Text>
                </ScrollView>
              </View>

              {/* Visibility Options */}
              <Text className="text-sm font-medium text-text-secondary mb-2">
                Who can see this?
              </Text>

              <TouchableOpacity
                className={`flex-row items-center rounded-lg p-3 mb-2 border ${
                  shareToFeedVisibility === 'friends_only'
                    ? 'border-primary-500 bg-primary-500/10'
                    : 'border-border-default bg-background-secondary'
                }`}
                onPress={() => setShareToFeedVisibility('friends_only')}
              >
                <Ionicons
                  name="people"
                  size={20}
                  color={shareToFeedVisibility === 'friends_only' ? colors.primary[500] : colors.text.secondary}
                  style={{ marginRight: spacing.sm }}
                />
                <View className="flex-1">
                  <Text className="text-sm font-medium text-text-primary">Friends Only</Text>
                  <Text className="text-xs text-text-tertiary">Only your friends can see this</Text>
                </View>
                {shareToFeedVisibility === 'friends_only' && (
                  <Ionicons name="checkmark-circle" size={20} color={colors.primary[500]} />
                )}
              </TouchableOpacity>

              <TouchableOpacity
                className={`flex-row items-center rounded-lg p-3 mb-4 border ${
                  shareToFeedVisibility === 'public'
                    ? 'border-primary-500 bg-primary-500/10'
                    : 'border-border-default bg-background-secondary'
                }`}
                onPress={() => setShareToFeedVisibility('public')}
              >
                <Ionicons
                  name="globe"
                  size={20}
                  color={shareToFeedVisibility === 'public' ? colors.primary[500] : colors.text.secondary}
                  style={{ marginRight: spacing.sm }}
                />
                <View className="flex-1">
                  <Text className="text-sm font-medium text-text-primary">Public</Text>
                  <Text className="text-xs text-text-tertiary">Anyone can see this</Text>
                </View>
                {shareToFeedVisibility === 'public' && (
                  <Ionicons name="checkmark-circle" size={20} color={colors.primary[500]} />
                )}
              </TouchableOpacity>

              {/* Action Buttons */}
              <TouchableOpacity
                className="rounded-lg p-3 mb-2 items-center"
                style={{ backgroundColor: colors.primary[600] }}
                onPress={handleShareToFeed}
                disabled={isSharing}
              >
                {isSharing ? (
                  <ActivityIndicator size="small" color={colors.text.primary} />
                ) : (
                  <Text className="text-base font-semibold text-text-primary">Share</Text>
                )}
              </TouchableOpacity>

              <TouchableOpacity
                className="items-center p-3"
                onPress={() => {
                  setShareToFeedContent(null);
                  setShareToFeedVisibility('friends_only');
                }}
              >
                <Text className="text-base text-text-tertiary">Cancel</Text>
              </TouchableOpacity>
            </View>
          </TouchableOpacity>
        </Modal>
      </KeyboardAvoidingView>
    </View>
  );
}
