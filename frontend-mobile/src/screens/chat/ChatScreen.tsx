// ABOUTME: Main chat screen with conversation list and message interface
// ABOUTME: Professional dark theme UI inspired by ChatGPT and Claude design

import React, { useState, useRef, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
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
} from 'react-native';
import * as Clipboard from 'expo-clipboard';
import * as Linking from 'expo-linking';
import * as WebBrowser from 'expo-web-browser';
import Markdown from 'react-native-markdown-display';
import { Ionicons } from '@expo/vector-icons';
import { useRoute, type RouteProp } from '@react-navigation/native';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Conversation, Message, ProviderStatus, Coach, CoachCategory } from '../../types';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

// Coach category badge background colors (lighter versions)
const COACH_CATEGORY_BADGE_BG: Record<CoachCategory, string> = {
  training: 'rgba(16, 185, 129, 0.15)',
  nutrition: 'rgba(245, 158, 11, 0.15)',
  recovery: 'rgba(99, 102, 241, 0.15)',
  recipes: 'rgba(249, 115, 22, 0.15)',
  custom: 'rgba(124, 58, 237, 0.15)',
};

// Coach category emoji icons
const COACH_CATEGORY_ICONS: Record<CoachCategory, string> = {
  training: '🏃',
  nutrition: '🥗',
  recovery: '😴',
  recipes: '👨‍🍳',
  custom: '⚙️',
};

interface ChatScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
}

export function ChatScreen({ navigation }: ChatScreenProps) {
  const { isAuthenticated } = useAuth();
  const route = useRoute<RouteProp<AppDrawerParamList, 'Chat'>>();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversation, setCurrentConversation] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [providerModalVisible, setProviderModalVisible] = useState(false);
  const [connectedProviders, setConnectedProviders] = useState<ProviderStatus[]>([]);
  const [pendingPrompt, setPendingPrompt] = useState<string | null>(null);
  const [messageFeedback, setMessageFeedback] = useState<Record<string, 'up' | 'down' | null>>({});
  const [coaches, setCoaches] = useState<Coach[]>([]);
  const [pendingCoachAction, setPendingCoachAction] = useState<{ coach: Coach } | null>(null);

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
      const response = await apiService.getOAuthStatus();
      setConnectedProviders(response.providers || []);
    } catch (error) {
      console.error('Failed to load provider status:', error);
    }
  };

  const loadCoaches = async () => {
    try {
      const response = await apiService.listCoaches();
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
      const response = await apiService.getConversations();
      // Deduplicate by ID to prevent duplicate key warnings
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      // Sort by updated_at descending (newest first)
      const sorted = deduplicated.sort((a, b) => {
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
      const response = await apiService.getConversationMessages(conversationId);
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
      await apiService.deleteConversation(conversationId);
      setConversations(prev => prev.filter(c => c.id !== conversationId));
      if (currentConversation?.id === conversationId) {
        setCurrentConversation(null);
      }
    } catch {
      Alert.alert('Error', 'Failed to delete conversation');
    }
  };

  const handleRenameConversation = (conversationId: string, currentTitle: string) => {
    Alert.prompt(
      'Rename Chat',
      'Enter a new name for this conversation',
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Save',
          onPress: async (newTitle: string | undefined) => {
            if (!newTitle?.trim()) {
              return;
            }
            try {
              const updated = await apiService.updateConversation(conversationId, {
                title: newTitle.trim(),
              });
              // Update conversation and move to top (most recently updated)
              setConversations(prev => {
                const updatedConv = prev.find(c => c.id === conversationId);
                if (!updatedConv) return prev;
                const others = prev.filter(c => c.id !== conversationId);
                return [
                  { ...updatedConv, title: updated.title, updated_at: updated.updated_at },
                  ...others,
                ];
              });
              // Always update currentConversation if IDs match
              setCurrentConversation(prev => {
                if (prev?.id === conversationId) {
                  return { ...prev, title: updated.title, updated_at: updated.updated_at };
                }
                return prev;
              });
            } catch (error) {
              console.error('Failed to rename conversation:', error);
              Alert.alert('Error', 'Failed to rename conversation');
            }
          },
        },
      ],
      'plain-text',
      currentTitle
    );
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

  const handleSendMessage = async () => {
    if (!inputText.trim() || isSending) return;

    const messageText = inputText.trim();
    setInputText('');
    setIsSending(true);

    // Create conversation if needed
    let conversationId = currentConversation?.id;
    if (!conversationId) {
      try {
        const conversation = await apiService.createConversation({
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
      const response = await apiService.sendMessage(conversationId, messageText);
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

  const handlePromptSelect = async (prompt: string) => {
    if (isSending) return;

    // Check if any provider is connected
    if (!hasConnectedProvider()) {
      setPendingPrompt(prompt);
      setProviderModalVisible(true);
      return;
    }

    await sendPromptMessage(prompt);
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
      apiService.recordCoachUsage(coach.id);

      // Create a new conversation with the coach's system prompt
      const conversation = await apiService.createConversation({
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
      const response = await apiService.sendMessage(conversation.id, initialMessage);

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
        const conversation = await apiService.createConversation({
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
      const response = await apiService.sendMessage(conversationId, prompt);
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
      const returnUrl = Linking.createURL('oauth-callback');

      // Call the mobile OAuth init endpoint which returns the authorization URL
      const oauthResponse = await apiService.initMobileOAuth(provider, returnUrl);

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
        <Text style={[styles.messageText, styles.userMessageText]}>
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
        <View style={styles.richTextContainer}>
          {oauthUrls.map((url, index) => {
            const { provider } = isOAuthUrl(url);
            return (
              <TouchableOpacity
                key={`oauth-${index}`}
                style={styles.oauthButton}
                onPress={() => handleOpenUrl(url)}
              >
                <Text style={styles.oauthButtonText}>
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

    return (
      <View style={[styles.messageContainer, isUser && styles.userMessageContainer]}>
        <View style={[
          styles.messageBubble,
          isUser ? styles.userBubble : styles.assistantBubble,
          isError && styles.errorBubble,
        ]}>
          {!isUser && (
            <View style={styles.assistantAvatarContainer}>
              <Image
                source={require('../../../assets/pierre-logo.png')}
                style={styles.assistantAvatarImage}
                resizeMode="cover"
              />
            </View>
          )}
          <View style={styles.messageContent}>
            {renderMessageContent(item.content, isUser)}
          </View>
        </View>
        {/* Action icons and metadata for assistant messages */}
        {!isUser && (
          <View style={styles.messageActions}>
            {isError ? (
              /* For error messages, show only Retry button */
              <TouchableOpacity
                style={styles.retryButton}
                onPress={() => handleRetryMessage(item.id)}
              >
                <Ionicons name="refresh-outline" size={14} color={colors.text.primary} />
                <Text style={styles.retryButtonText}>Retry</Text>
              </TouchableOpacity>
            ) : (
              /* Normal assistant message actions */
              <>
                {/* Copy */}
                <TouchableOpacity
                  style={styles.messageActionButton}
                  onPress={() => handleCopyMessage(item.content)}
                >
                  <Ionicons name="copy-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Share */}
                <TouchableOpacity
                  style={styles.messageActionButton}
                  onPress={() => handleShareMessage(item.content)}
                >
                  <Ionicons name="arrow-redo-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Thumbs Up */}
                <TouchableOpacity
                  style={styles.messageActionButton}
                  onPress={() => handleThumbsUp(item.id)}
                >
                  <Ionicons
                    name={messageFeedback[item.id] === 'up' ? 'thumbs-up' : 'thumbs-up-outline'}
                    size={14}
                    color={messageFeedback[item.id] === 'up' ? colors.primary[400] : colors.text.tertiary}
                  />
                </TouchableOpacity>
                {/* Thumbs Down */}
                <TouchableOpacity
                  style={styles.messageActionButton}
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
                  style={styles.messageActionButton}
                  onPress={() => handleRetryMessage(item.id)}
                >
                  <Ionicons name="refresh-outline" size={14} color={colors.text.tertiary} />
                </TouchableOpacity>
                {/* Model and response time - to the right of icons */}
                {item.model && (
                  <Text style={styles.messageMetadata}>
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
    <View style={styles.messageContainer}>
      <View style={[styles.messageBubble, styles.assistantBubble]}>
        <View style={styles.assistantAvatarContainer}>
          <Image
            source={require('../../../assets/pierre-logo.png')}
            style={styles.assistantAvatarImage}
            resizeMode="cover"
          />
        </View>
        <View style={styles.thinkingContent}>
          <ActivityIndicator size="small" color={colors.primary[500]} style={styles.thinkingSpinner} />
          <Text style={styles.thinkingText}>Pierre is thinking...</Text>
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
      style={styles.coachGridCard}
      onPress={() => handleCoachSelect(coach)}
      activeOpacity={0.7}
    >
      {/* Header row: Title (full width, wraps to 2 lines) + Category icon */}
      <View style={styles.coachCardHeader}>
        <Text style={styles.coachTitle} numberOfLines={2}>
          {coach.title}
        </Text>
        <View style={[
          styles.coachCategoryBadge,
          { backgroundColor: COACH_CATEGORY_BADGE_BG[coach.category] },
        ]}>
          <Text style={styles.coachCategoryIcon}>
            {COACH_CATEGORY_ICONS[coach.category]}
          </Text>
        </View>
      </View>
      {/* Description */}
      {coach.description && (
        <Text style={styles.coachDescription} numberOfLines={2}>
          {coach.description}
        </Text>
      )}
      {/* Footer: Badges + Use count */}
      <View style={styles.coachCardFooter}>
        {coach.is_system && (
          <View style={styles.systemBadge}>
            <Text style={styles.systemBadgeText}>System</Text>
          </View>
        )}
        {coach.is_favorite && (
          <View style={styles.favoriteBadge}>
            <Text style={styles.favoriteBadgeIcon}>★</Text>
          </View>
        )}
        <View style={styles.footerSpacer} />
        {coach.use_count > 0 && (
          <Text style={styles.coachUseCount}>{coach.use_count}×</Text>
        )}
      </View>
    </TouchableOpacity>
  );

  const renderEmptyChat = () => (
    <ScrollView
      style={styles.emptyScrollView}
      contentContainerStyle={styles.emptyContainer}
      showsVerticalScrollIndicator={false}
      keyboardShouldPersistTaps="handled"
    >
      {/* Coach Grid - only show when NOT in a coach conversation */}
      {!isCoachConversation && coaches.length > 0 && (
        <View style={styles.coachGridContainer}>
          <Text style={styles.coachGridTitle}>🎯 Your Coaches</Text>
          <View style={styles.coachGrid}>
            {coaches.map((coach) => renderCoachGridCard(coach))}
          </View>
        </View>
      )}

      {/* Empty state when no coaches */}
      {!isCoachConversation && coaches.length === 0 && (
        <View style={styles.noCoachesContainer}>
          <Text style={styles.noCoachesTitle}>No coaches yet</Text>
          <Text style={styles.noCoachesSubtitle}>
            Create your first coach to customize how Pierre helps you.
          </Text>
        </View>
      )}

      {/* Coach conversation starter */}
      {isCoachConversation && (
        <View style={styles.coachStarterContainer}>
          <Text style={styles.coachStarterText}>
            Your coach is ready. Start the conversation by typing a message or try one of these:
          </Text>
          <TouchableOpacity
            style={styles.suggestionButton}
            onPress={() => handlePromptSelect("Let's get started!")}
            activeOpacity={0.6}
          >
            <Text style={styles.suggestionText}>Let's get started!</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={styles.suggestionButton}
            onPress={() => handlePromptSelect("What can you help me with?")}
            activeOpacity={0.6}
          >
            <Text style={styles.suggestionText}>What can you help me with?</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={styles.suggestionButton}
            onPress={() => handlePromptSelect("Give me today's plan")}
            activeOpacity={0.6}
          >
            <Text style={styles.suggestionText}>Give me today's plan</Text>
          </TouchableOpacity>
        </View>
      )}

    </ScrollView>
  );

  return (
    <SafeAreaView style={styles.container} testID="chat-screen">
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
        keyboardVerticalOffset={Platform.OS === 'ios' ? 0 : 0}
      >
        {/* Header */}
        <View style={styles.header}>
          <TouchableOpacity
            style={styles.menuButton}
            onPress={() => navigation.openDrawer()}
            testID="menu-button"
          >
            <Text style={styles.menuIcon}>{'☰'}</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={[styles.headerTitleContainer, actionMenuVisible && styles.headerTitleHidden]}
            onPress={showTitleActionMenu}
            disabled={!currentConversation}
            testID="chat-title-button"
          >
            <Text style={styles.headerTitle} numberOfLines={1} testID="chat-title">
              {currentConversation?.title || 'New Chat'}
            </Text>
            {currentConversation && (
              <Text style={styles.chevronIcon}>▼</Text>
            )}
          </TouchableOpacity>
          <TouchableOpacity style={styles.newChatButton} onPress={handleNewChat} testID="new-chat-button">
            <Text style={styles.newChatIcon}>+</Text>
          </TouchableOpacity>
        </View>

        {/* Messages or Empty State */}
        {isLoading ? (
          <View style={styles.loadingContainer}>
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
            contentContainerStyle={styles.messagesList}
            showsVerticalScrollIndicator={false}
            onContentSizeChange={scrollToBottom}
            ListFooterComponent={isSending ? renderThinkingIndicator : null}
          />
        )}

        {/* Input Area */}
        <View style={styles.inputContainer}>
          <View style={styles.inputWrapper}>
            <TextInput
              ref={inputRef}
              style={styles.input}
              placeholder="Message Pierre..."
              placeholderTextColor={colors.text.tertiary}
              value={inputText}
              onChangeText={setInputText}
              multiline
              maxLength={4000}
              returnKeyType="default"
              testID="message-input"
            />
            <TouchableOpacity
              style={[
                styles.sendButton,
                (!inputText.trim() || isSending) && styles.sendButtonDisabled,
              ]}
              onPress={handleSendMessage}
              disabled={!inputText.trim() || isSending}
              testID="send-button"
            >
              {isSending ? (
                <ActivityIndicator size="small" color={colors.text.primary} />
              ) : (
                <Text style={styles.sendIcon}>{'>'}</Text>
              )}
            </TouchableOpacity>
          </View>
        </View>

        {/* Conversation Action Menu Modal - Claude-style popover */}
        <Modal
          visible={actionMenuVisible}
          animationType="fade"
          transparent
          onRequestClose={closeActionMenu}
        >
          <TouchableOpacity
            style={styles.popoverOverlay}
            activeOpacity={1}
            onPress={closeActionMenu}
          >
            <View style={styles.popoverContainer}>
              <TouchableOpacity
                style={[styles.popoverItem, styles.popoverItemDisabled]}
                disabled
              >
                <Ionicons name="star-outline" size={20} color={colors.text.tertiary} style={styles.popoverIcon} />
                <Text style={styles.popoverTextDisabled}>Add to favorites</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.popoverItem}
                onPress={handleMenuRename}
              >
                <Ionicons name="pencil-outline" size={20} color={colors.text.primary} style={styles.popoverIcon} />
                <Text style={styles.popoverText}>Rename</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.popoverItem}
                onPress={handleMenuDelete}
              >
                <Ionicons name="trash-outline" size={20} color={colors.error} style={styles.popoverIcon} />
                <Text style={styles.popoverTextDanger}>Delete</Text>
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
            style={styles.modalOverlay}
            activeOpacity={1}
            onPress={() => {
              setProviderModalVisible(false);
              setPendingPrompt(null);
            }}
          >
            <View style={styles.providerModalContainer}>
              <Text style={styles.providerModalTitle}>Connect a Provider</Text>
              <Text style={styles.providerModalSubtitle}>
                To analyze your fitness data, please connect a provider first.
              </Text>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('strava')}
              >
                <Text style={styles.providerButtonIcon}>🚴</Text>
                <Text style={styles.providerButtonText}>Connect Strava</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('fitbit')}
              >
                <Text style={styles.providerButtonIcon}>⌚</Text>
                <Text style={styles.providerButtonText}>Connect Fitbit</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('garmin')}
              >
                <Text style={styles.providerButtonIcon}>⌚</Text>
                <Text style={styles.providerButtonText}>Connect Garmin</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('whoop')}
              >
                <Text style={styles.providerButtonIcon}>💪</Text>
                <Text style={styles.providerButtonText}>Connect WHOOP</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('coros')}
              >
                <Text style={styles.providerButtonIcon}>🏃</Text>
                <Text style={styles.providerButtonText}>Connect COROS</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerButton}
                onPress={() => handleConnectProvider('terra')}
              >
                <Text style={styles.providerButtonIcon}>🌍</Text>
                <Text style={styles.providerButtonText}>Connect Terra</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.providerCancelButton}
                onPress={() => {
                  setProviderModalVisible(false);
                  setPendingPrompt(null);
                  setPendingCoachAction(null);
                }}
              >
                <Text style={styles.providerCancelText}>Cancel</Text>
              </TouchableOpacity>
            </View>
          </TouchableOpacity>
        </Modal>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
  },
  keyboardView: {
    flex: 1,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  menuButton: {
    width: 40,
    height: 40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  menuIcon: {
    fontSize: 20,
    color: colors.text.primary,
  },
  headerTitleContainer: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    marginHorizontal: spacing.sm,
  },
  headerTitleHidden: {
    opacity: 0,
  },
  headerTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
  },
  chevronIcon: {
    fontSize: 10,
    marginLeft: spacing.xs,
    color: colors.text.tertiary,
  },
  newChatButton: {
    width: 40,
    height: 40,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.background.tertiary,
    borderRadius: borderRadius.md,
  },
  newChatIcon: {
    fontSize: 24,
    color: colors.text.primary,
    fontWeight: '300',
  },
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  messagesList: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    paddingBottom: 80, // Space for floating input overlay
  },
  messageContainer: {
    marginBottom: spacing.md,
  },
  userMessageContainer: {
    alignItems: 'flex-end',
  },
  messageBubble: {
    flexDirection: 'row',
    maxWidth: '85%',
    borderRadius: borderRadius.lg,
    padding: spacing.md,
  },
  userBubble: {
    backgroundColor: colors.primary[600],
    borderBottomRightRadius: 4,
  },
  assistantBubble: {
    backgroundColor: colors.background.secondary,
    borderBottomLeftRadius: 4,
  },
  errorBubble: {
    backgroundColor: 'rgba(239, 68, 68, 0.15)',
    borderColor: colors.error,
    borderWidth: 1,
  },
  assistantAvatarContainer: {
    width: 32,
    height: 32,
    borderRadius: 16,
    marginRight: spacing.sm,
    overflow: 'hidden',
  },
  assistantAvatarImage: {
    width: 32,
    height: 32,
  },
  messageContent: {
    flex: 1,
  },
  messageText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
    lineHeight: 22,
  },
  userMessageText: {
    color: colors.text.primary,
  },
  richTextContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    alignItems: 'center',
  },
  linkText: {
    fontSize: fontSize.md,
    color: colors.primary[400],
    textDecorationLine: 'underline',
    lineHeight: 22,
  },
  oauthButton: {
    backgroundColor: colors.providers.strava,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderRadius: borderRadius.md,
    marginVertical: spacing.xs,
    alignSelf: 'flex-start',
  },
  oauthButtonText: {
    color: colors.text.primary,
    fontSize: fontSize.md,
    fontWeight: '600',
  },
  emptyScrollView: {
    flex: 1,
  },
  emptyContainer: {
    flexGrow: 1,
    alignItems: 'center',
    justifyContent: 'flex-start',
    paddingHorizontal: spacing.xs,
    paddingVertical: spacing.md,
    paddingBottom: 100, // Space for floating input overlay
  },
  welcomeLogo: {
    width: 120,
    height: 120,
    marginBottom: spacing.md,
  },
  welcomeTitle: {
    fontSize: fontSize.xxl,
    fontWeight: '700',
    color: colors.text.primary,
    marginBottom: spacing.xs,
  },
  welcomeSubtitle: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
    marginBottom: spacing.lg,
    lineHeight: 22,
  },
  welcomePromptContainer: {
    width: '100%',
    maxWidth: 400,
    marginBottom: spacing.md,
  },
  suggestionsContainer: {
    width: '100%',
    maxWidth: 400,
  },
  categoryContainer: {
    marginBottom: spacing.md,
  },
  categoryTitle: {
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.secondary,
    marginBottom: spacing.xs,
  },
  suggestionButton: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.md,
    padding: spacing.sm,
    marginBottom: spacing.xs,
    borderWidth: 1,
    borderColor: colors.border.subtle,
  },
  suggestionText: {
    fontSize: fontSize.sm,
    color: colors.text.primary,
    lineHeight: 20,
  },
  // Coach Carousel Styles
  coachGridContainer: {
    width: '100%',
    paddingHorizontal: spacing.xs,
  },
  coachGridTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.md,
  },
  coachGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    justifyContent: 'space-between',
    gap: spacing.sm,
  },
  coachGridCard: {
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.lg,
    padding: spacing.md,
    width: '48%',
    borderWidth: 1,
    borderColor: colors.border.subtle,
    marginBottom: spacing.sm,
  },
  coachCardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: spacing.xs,
    gap: spacing.sm,
  },
  coachCategoryBadge: {
    width: 28,
    height: 28,
    borderRadius: borderRadius.sm,
    alignItems: 'center',
    justifyContent: 'center',
  },
  coachCategoryIcon: {
    fontSize: 14,
  },
  coachCardFooter: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
    marginTop: spacing.xs,
  },
  systemBadge: {
    backgroundColor: 'rgba(124, 58, 237, 0.15)',
    paddingHorizontal: spacing.sm,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
  },
  systemBadgeText: {
    fontSize: fontSize.xs,
    fontWeight: '500',
    color: '#7C3AED',
  },
  favoriteBadge: {
    backgroundColor: 'rgba(245, 158, 11, 0.15)',
    paddingHorizontal: spacing.xs,
    paddingVertical: 2,
    borderRadius: borderRadius.sm,
  },
  favoriteBadgeIcon: {
    fontSize: fontSize.xs,
    color: '#F59E0B',
  },
  footerSpacer: {
    flex: 1,
  },
  coachUseCount: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
  },
  noCoachesContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.xxl,
  },
  noCoachesTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    marginBottom: spacing.sm,
  },
  noCoachesSubtitle: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
    textAlign: 'center',
  },
  coachTitle: {
    flex: 1,
    fontSize: fontSize.sm,
    fontWeight: '600',
    color: colors.text.primary,
    lineHeight: 18,
  },
  coachDescription: {
    fontSize: fontSize.xs,
    color: colors.text.secondary,
    lineHeight: 16,
    marginBottom: spacing.xs,
  },
  coachStarterContainer: {
    width: '100%',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    marginBottom: spacing.lg,
  },
  coachStarterText: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    textAlign: 'center',
    marginBottom: spacing.lg,
    lineHeight: 22,
  },
  inputContainer: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    backgroundColor: colors.background.primary,
  },
  inputWrapper: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.xl,
    borderWidth: 1,
    borderColor: colors.border.default,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    minHeight: 48,
    maxHeight: 120,
  },
  input: {
    flex: 1,
    fontSize: fontSize.md,
    color: colors.text.primary,
    paddingVertical: spacing.sm,
    maxHeight: 100,
  },
  sendButton: {
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: colors.primary[600],
    alignItems: 'center',
    justifyContent: 'center',
    marginLeft: spacing.sm,
  },
  sendButtonDisabled: {
    backgroundColor: colors.background.tertiary,
  },
  sendIcon: {
    fontSize: 18,
    color: colors.text.primary,
    fontWeight: '700',
  },
  thinkingContent: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  thinkingSpinner: {
    marginRight: spacing.sm,
  },
  thinkingText: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
    fontStyle: 'italic',
  },
  messageActions: {
    flexDirection: 'row',
    marginTop: spacing.xs,
    marginLeft: 0, // Far left, no padding
    gap: spacing.md,
  },
  messageActionButton: {
    padding: 2,
  },
  retryButton: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.background.tertiary,
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xs,
    borderRadius: borderRadius.sm,
    gap: spacing.xs,
  },
  retryButtonText: {
    fontSize: fontSize.xs,
    color: colors.text.primary,
    fontWeight: '500',
  },
  messageMetadata: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    marginLeft: spacing.sm,
  },
  // Centered modal overlay (for provider selection)
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  // Claude-style popover dropdown (dark theme)
  popoverOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.3)',
  },
  popoverContainer: {
    position: 'absolute',
    top: 68, // Align with + button background top
    left: 60, // Equal margins to center
    right: 60, // Equal margins to center
    backgroundColor: colors.background.secondary,
    borderRadius: 12,
    paddingVertical: spacing.xs,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.4,
    shadowRadius: 16,
    elevation: 12,
  },
  popoverItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: 12,
  },
  popoverItemDisabled: {
    opacity: 0.4,
  },
  popoverIcon: {
    marginRight: spacing.md,
    width: 24,
  },
  popoverText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
    fontWeight: '400',
  },
  popoverTextDisabled: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
    fontWeight: '400',
  },
  popoverTextDanger: {
    fontSize: fontSize.md,
    color: colors.error,
    fontWeight: '400',
  },
  providerModalContainer: {
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
  },
  providerModalTitle: {
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
    marginBottom: spacing.xs,
  },
  providerModalSubtitle: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
    textAlign: 'center',
    marginBottom: spacing.lg,
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
  providerButtonIcon: {
    fontSize: 24,
    marginRight: spacing.md,
  },
  providerButtonText: {
    fontSize: fontSize.md,
    color: colors.text.primary,
    fontWeight: '500',
  },
  providerCancelButton: {
    alignItems: 'center',
    padding: spacing.md,
    marginTop: spacing.xs,
  },
  providerCancelText: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
  },
});
