// ABOUTME: Main chat screen with conversation list and message interface
// ABOUTME: Professional dark theme UI inspired by ChatGPT and Claude design

import React, { useState, useRef, useCallback, useEffect } from 'react';
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
  Animated,
  Dimensions,
  ActivityIndicator,
  Alert,
  Image,
  ScrollView,
  Linking,
} from 'react-native';
import * as WebBrowser from 'expo-web-browser';
import Markdown from 'react-native-markdown-display';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Conversation, Message, PromptCategory } from '../../types';
import type { DrawerNavigationProp } from '@react-navigation/drawer';

const { width: SCREEN_WIDTH } = Dimensions.get('window');

interface ChatScreenProps {
  navigation: DrawerNavigationProp<Record<string, undefined>>;
}

// Default prompts - used as initial state and fallback when API is unavailable
const DEFAULT_PROMPT_CATEGORIES: PromptCategory[] = [
  {
    category_key: 'training',
    category_title: 'Training',
    category_icon: '🏃',
    pillar: 'activity',
    prompts: [
      'Am I ready for a hard workout today?',
      "What's my predicted marathon time?",
    ],
  },
  {
    category_key: 'nutrition',
    category_title: 'Nutrition',
    category_icon: '🥗',
    pillar: 'nutrition',
    prompts: [
      'How many calories should I eat today?',
      'What should I eat before my morning run?',
    ],
  },
  {
    category_key: 'recovery',
    category_title: 'Recovery',
    category_icon: '🧘',
    pillar: 'recovery',
    prompts: [
      'Do I need a rest day?',
      'Analyze my sleep quality',
    ],
  },
];

const DEFAULT_WELCOME_PROMPT = 'Show my recent activities';

// Metadata for assistant messages (model name and execution time)
interface MessageMetadata {
  model: string;
  executionTimeMs: number;
}

export function ChatScreen({ navigation }: ChatScreenProps) {
  const { isAuthenticated } = useAuth();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversation, setCurrentConversation] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [promptCategories, setPromptCategories] = useState<PromptCategory[]>(DEFAULT_PROMPT_CATEGORIES);
  const [welcomePrompt, setWelcomePrompt] = useState(DEFAULT_WELCOME_PROMPT);
  // Track model and execution time for each assistant message
  const [messageMetadata, setMessageMetadata] = useState<Map<string, MessageMetadata>>(new Map());

  const flatListRef = useRef<FlatList>(null);
  const inputRef = useRef<TextInput>(null);

  // Load conversations and prompts only when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      loadConversations();
      loadPromptSuggestions();
    }
  }, [isAuthenticated]);

  // Load messages when conversation changes
  useEffect(() => {
    if (currentConversation) {
      loadMessages(currentConversation.id);
    } else {
      setMessages([]);
    }
  }, [currentConversation]);

  const loadConversations = async () => {
    try {
      setIsLoading(true);
      const response = await apiService.getConversations();
      setConversations(response.conversations);
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

  const loadPromptSuggestions = async () => {
    try {
      const response = await apiService.getPromptSuggestions();
      if (response.categories && response.categories.length > 0) {
        setPromptCategories(response.categories);
      }
      if (response.welcome_prompt) {
        setWelcomePrompt(response.welcome_prompt);
      }
    } catch (error) {
      console.error('Failed to load prompts:', error);
      // Keep default prompts on error
    }
  };

  const scrollToBottom = () => {
    if (flatListRef.current && messages.length > 0) {
      flatListRef.current.scrollToEnd({ animated: true });
    }
  };

  const handleNewChat = async () => {
    try {
      const conversation = await apiService.createConversation({
        title: 'New Chat',
      });
      setConversations(prev => [conversation, ...prev]);
      setCurrentConversation(conversation);
    } catch (error) {
      Alert.alert('Error', 'Failed to create new conversation');
    }
  };

  const handleSelectConversation = (conversation: Conversation) => {
    setCurrentConversation(conversation);
    navigation.closeDrawer();
  };

  const handleDeleteConversation = async (conversationId: string) => {
    try {
      await apiService.deleteConversation(conversationId);
      setConversations(prev => prev.filter(c => c.id !== conversationId));
      if (currentConversation?.id === conversationId) {
        setCurrentConversation(null);
      }
    } catch (error) {
      Alert.alert('Error', 'Failed to delete conversation');
    }
  };

  const handleSendMessage = async () => {
    if (!inputText.trim() || isSending) return;

    const messageText = inputText.trim();
    setInputText('');

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
        setCurrentConversation(conversation);
        conversationId = conversation.id;
      } catch (error) {
        console.error('Failed to create conversation:', error);
        Alert.alert('Error', 'Failed to create conversation');
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

    setIsSending(true);

    try {
      const response = await apiService.sendMessage(conversationId, messageText);
      // Replace optimistic user message and add assistant response
      setMessages(prev => {
        const filtered = prev.filter(m => m.id !== userMessage.id);
        return [...filtered, response.user_message, response.assistant_message];
      });
      // Store model and execution time metadata for the assistant message
      if (response.assistant_message.id && (response.model || response.execution_time_ms)) {
        setMessageMetadata(prev => {
          const updated = new Map(prev);
          updated.set(response.assistant_message.id, {
            model: response.model || 'unknown',
            executionTimeMs: response.execution_time_ms || 0,
          });
          return updated;
        });
      }
      scrollToBottom();
    } catch (error) {
      console.error('Failed to send message:', error);
      Alert.alert('Error', 'Failed to send message. Please try again.');
      // Remove optimistic message on failure
      setMessages(prev => prev.filter(m => m.id !== userMessage.id));
    } finally {
      setIsSending(false);
    }
  };

  const handlePromptSelect = async (prompt: string) => {
    if (isSending) return;

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
        setCurrentConversation(conversation);
        conversationId = conversation.id;
      } catch (error) {
        console.error('Failed to create conversation:', error);
        Alert.alert('Error', 'Failed to create conversation');
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

    setIsSending(true);

    try {
      const response = await apiService.sendMessage(conversationId, prompt);
      // Replace optimistic user message and add assistant response
      setMessages(prev => {
        const filtered = prev.filter(m => m.id !== userMessage.id);
        return [...filtered, response.user_message, response.assistant_message];
      });
      // Store model and execution time metadata for the assistant message
      if (response.assistant_message.id && (response.model || response.execution_time_ms)) {
        setMessageMetadata(prev => {
          const updated = new Map(prev);
          updated.set(response.assistant_message.id, {
            model: response.model || 'unknown',
            executionTimeMs: response.execution_time_ms || 0,
          });
          return updated;
        });
      }
      scrollToBottom();
    } catch (error) {
      console.error('Failed to send message:', error);
      Alert.alert('Error', 'Failed to send message. Please try again.');
      // Remove optimistic message on failure
      setMessages(prev => prev.filter(m => m.id !== userMessage.id));
    } finally {
      setIsSending(false);
    }
  };

  // Helper to detect OAuth authorization URLs
  const isOAuthUrl = (url: string): { isOAuth: boolean; provider: string | null } => {
    if (url.includes('strava.com/oauth/authorize')) {
      return { isOAuth: true, provider: 'Strava' };
    }
    if (url.includes('fitbit.com/oauth2/authorize')) {
      return { isOAuth: true, provider: 'Fitbit' };
    }
    if (url.includes('garmin.com') && url.includes('oauth')) {
      return { isOAuth: true, provider: 'Garmin' };
    }
    return { isOAuth: false, provider: null };
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
      color: colors.text,
      fontSize: fontSize.body,
      lineHeight: fontSize.body * 1.5,
    },
    heading1: {
      color: colors.text,
      fontSize: fontSize.title,
      fontWeight: '700' as const,
      marginTop: spacing.md,
      marginBottom: spacing.sm,
    },
    heading2: {
      color: colors.text,
      fontSize: fontSize.large,
      fontWeight: '600' as const,
      marginTop: spacing.sm,
      marginBottom: spacing.xs,
    },
    heading3: {
      color: colors.text,
      fontSize: fontSize.body,
      fontWeight: '600' as const,
      marginTop: spacing.xs,
      marginBottom: spacing.xs,
    },
    strong: {
      color: colors.text,
      fontWeight: '700' as const,
    },
    em: {
      color: colors.textSecondary,
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
      backgroundColor: colors.inputBackground,
      color: colors.primary,
      paddingHorizontal: 4,
      borderRadius: 4,
      fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
      fontSize: fontSize.small,
    },
    fence: {
      backgroundColor: colors.inputBackground,
      borderRadius: borderRadius.sm,
      padding: spacing.sm,
      marginVertical: spacing.xs,
    },
    code_block: {
      fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
      fontSize: fontSize.small,
      color: colors.text,
    },
    link: {
      color: colors.primary,
      textDecorationLine: 'underline' as const,
    },
    hr: {
      backgroundColor: colors.border,
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
        // Remove the URL and any markdown link syntax around it
        cleanContent = cleanContent.replace(new RegExp(`\\[([^\\]]*)\\]\\(${url.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\)`, 'g'), '');
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
    const isUser = item.role === 'user';
    const metadata = messageMetadata.get(item.id);

    return (
      <View style={[styles.messageContainer, isUser && styles.userMessageContainer]}>
        <View style={[styles.messageBubble, isUser ? styles.userBubble : styles.assistantBubble]}>
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
            {/* Model and execution time for assistant messages */}
            {!isUser && metadata && (
              <View style={styles.messageMetadata}>
                <Text style={styles.metadataText}>
                  {metadata.model} • {(metadata.executionTimeMs / 1000).toFixed(1)}s
                </Text>
              </View>
            )}
          </View>
        </View>
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

  const renderEmptyChat = () => (
    <ScrollView
      style={styles.emptyScrollView}
      contentContainerStyle={styles.emptyContainer}
      showsVerticalScrollIndicator={false}
    >
      <Image
        source={require('../../../assets/pierre-logo.png')}
        style={styles.welcomeLogo}
        resizeMode="contain"
      />
      <Text style={styles.welcomeTitle}>Pierre</Text>
      <Text style={styles.welcomeSubtitle}>
        Real sports science to help you train better, eat smarter, and recover faster.
      </Text>
      <View style={styles.welcomePromptContainer}>
        <Text style={styles.categoryTitle}>
          📊 Quick Start
        </Text>
        <TouchableOpacity
          style={styles.suggestionButton}
          onPress={() => handlePromptSelect(welcomePrompt)}
        >
          <Text style={styles.suggestionText}>
            {welcomePrompt}
          </Text>
        </TouchableOpacity>
      </View>

      {/* Prompt Suggestions */}
      <View style={styles.suggestionsContainer}>
        {promptCategories.slice(0, 3).map((category) => (
          <View key={category.category_key} style={styles.categoryContainer}>
            <Text style={styles.categoryTitle}>
              {category.category_icon} {category.category_title}
            </Text>
            {category.prompts.slice(0, 2).map((prompt, promptIndex) => (
              <TouchableOpacity
                key={`${category.category_key}-prompt-${promptIndex}`}
                style={styles.suggestionButton}
                onPress={() => handlePromptSelect(prompt)}
              >
                <Text style={styles.suggestionText} numberOfLines={2}>
                  {prompt}
                </Text>
              </TouchableOpacity>
            ))}
          </View>
        ))}
      </View>
    </ScrollView>
  );

  return (
    <SafeAreaView style={styles.container}>
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
          >
            <Text style={styles.menuIcon}>{'☰'}</Text>
          </TouchableOpacity>
          <Text style={styles.headerTitle} numberOfLines={1}>
            {currentConversation?.title || 'New Chat'}
          </Text>
          <TouchableOpacity style={styles.newChatButton} onPress={handleNewChat}>
            <Text style={styles.newChatIcon}>+</Text>
          </TouchableOpacity>
        </View>

        {/* Messages or Empty State */}
        {isLoading ? (
          <View style={styles.loadingContainer}>
            <ActivityIndicator size="large" color={colors.primary[500]} />
          </View>
        ) : messages.length === 0 ? (
          renderEmptyChat()
        ) : (
          <FlatList
            ref={flatListRef}
            data={messages}
            renderItem={renderMessage}
            keyExtractor={(item, index) => `${item.id}-${index}`}
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
            />
            <TouchableOpacity
              style={[
                styles.sendButton,
                (!inputText.trim() || isSending) && styles.sendButtonDisabled,
              ]}
              onPress={handleSendMessage}
              disabled={!inputText.trim() || isSending}
            >
              {isSending ? (
                <ActivityIndicator size="small" color={colors.text.primary} />
              ) : (
                <Text style={styles.sendIcon}>{'>'}</Text>
              )}
            </TouchableOpacity>
          </View>
        </View>
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
  headerTitle: {
    flex: 1,
    fontSize: fontSize.lg,
    fontWeight: '600',
    color: colors.text.primary,
    textAlign: 'center',
    marginHorizontal: spacing.sm,
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
    backgroundColor: '#FC4C02', // Strava orange as default
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
    justifyContent: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.xl,
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
  inputContainer: {
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
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
  messageMetadata: {
    marginTop: spacing.sm,
    paddingTop: spacing.xs,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  metadataText: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
  },
});
