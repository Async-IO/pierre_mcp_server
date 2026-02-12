// ABOUTME: Full conversations list screen with search functionality
// ABOUTME: Shows all conversations with relative dates and search bar

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  TextInput,
  ActivityIndicator,
  Alert,
  Modal,
  type ViewStyle,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Feather } from '@expo/vector-icons';
import { LinearGradient } from 'expo-linear-gradient';
import { colors, spacing, glassCard, gradients, buttonGlow } from '../../constants/theme';
import { chatApi } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { PromptDialog, SwipeableRow, type SwipeAction } from '../../components/ui';
import type { Conversation } from '../../types';
import type { ChatStackParamList } from '../../navigation/MainTabs';

interface ConversationsScreenProps {
  navigation: NativeStackNavigationProp<ChatStackParamList>;
}

// Glassmorphic search bar style
const searchBarStyle: ViewStyle = {
  ...glassCard,
  borderRadius: 22,
  borderColor: 'rgba(139, 92, 246, 0.2)',
};

// FAB with violet glow
const fabStyle: ViewStyle = {
  backgroundColor: colors.pierre.violet,
  ...buttonGlow,
};

// Glassmorphic menu style
const menuStyle: ViewStyle = {
  ...glassCard,
  borderRadius: 16,
  borderColor: 'rgba(139, 92, 246, 0.2)',
};

export function ConversationsScreen({ navigation }: ConversationsScreenProps) {
  const { isAuthenticated } = useAuth();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [filteredConversations, setFilteredConversations] = useState<Conversation[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(null);
  const [renamePromptVisible, setRenamePromptVisible] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoading(true);
      setError(null);
      const response = await chatApi.getConversations();
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv: { id: string }) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      const sorted = deduplicated.sort(
        (a: { updated_at: string }, b: { updated_at: string }) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      );
      setConversations(sorted);
      setFilteredConversations(sorted);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load conversations';
      setError(errorMessage);
      console.error('Failed to load conversations:', err);
    } finally {
      setIsLoading(false);
    }
  }, [isAuthenticated]);

  useFocusEffect(
    useCallback(() => {
      loadConversations();
    }, [loadConversations])
  );

  useEffect(() => {
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      const filtered = conversations.filter((conv) =>
        (conv.title || '').toLowerCase().includes(query)
      );
      setFilteredConversations(filtered);
    } else {
      setFilteredConversations(conversations);
    }
  }, [searchQuery, conversations]);

  const formatRelativeDate = (dateString: string): string => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    const diffWeeks = Math.floor(diffDays / 7);
    const diffMonths = Math.floor(diffDays / 30);

    if (diffDays === 0) {
      return 'today';
    } else if (diffDays === 1) {
      return 'yesterday';
    } else if (diffDays < 7) {
      return `${diffDays} days ago`;
    } else if (diffWeeks === 1) {
      return '1 week ago';
    } else if (diffWeeks < 4) {
      return `${diffWeeks} weeks ago`;
    } else if (diffMonths === 1) {
      return '1 month ago';
    } else {
      return `${diffMonths} months ago`;
    }
  };

  const handleConversationPress = (conversationId: string) => {
    navigation.navigate('ChatMain', { conversationId });
  };

  const handleConversationLongPress = (conversation: Conversation) => {
    setSelectedConversation(conversation);
    setActionMenuVisible(true);
  };

  const handleNewChat = () => {
    navigation.navigate('ChatMain', { conversationId: undefined });
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
      const updated = await chatApi.updateConversation(selectedConversation.id, {
        title: newTitle,
      });
      setConversations((prev) =>
        prev.map((c) => (c.id === selectedConversation.id ? { ...c, title: updated.title } : c))
      );
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to rename conversation';
      setError(errorMessage);
      console.error('Failed to rename conversation:', err);
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
              await chatApi.deleteConversation(selectedConversation.id);
              setConversations((prev) => prev.filter((c) => c.id !== selectedConversation.id));
            } catch (err) {
              const errorMessage = err instanceof Error ? err.message : 'Failed to delete conversation';
              setError(errorMessage);
              console.error('Failed to delete conversation:', err);
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

  const renderConversation = ({ item }: { item: Conversation }) => {
    const leftActions: SwipeAction[] = [
      {
        icon: 'edit-2',
        label: 'Rename',
        color: '#FFFFFF',
        backgroundColor: colors.pierre.violet,
        onPress: () => {
          setSelectedConversation(item);
          setRenamePromptVisible(true);
        },
      },
    ];

    const rightActions: SwipeAction[] = [
      {
        icon: 'trash-2',
        label: 'Delete',
        color: '#FFFFFF',
        backgroundColor: '#EF4444',
        onPress: () => {
          setSelectedConversation(item);
          Alert.alert(
            'Delete Conversation',
            `Are you sure you want to delete "${item.title || 'this conversation'}"?`,
            [
              { text: 'Cancel', style: 'cancel' },
              {
                text: 'Delete',
                style: 'destructive',
                onPress: async () => {
                  try {
                    await chatApi.deleteConversation(item.id);
                    setConversations((prev) => prev.filter((c) => c.id !== item.id));
                  } catch (err) {
                    const errorMessage = err instanceof Error ? err.message : 'Failed to delete conversation';
                    setError(errorMessage);
                    console.error('Failed to delete conversation:', err);
                  }
                },
              },
            ]
          );
        },
      },
    ];

    return (
      <SwipeableRow
        leftActions={leftActions}
        rightActions={rightActions}
        testID={`swipeable-conversation-${item.id}`}
      >
        <TouchableOpacity
          className="flex-row items-center px-4 py-3 border-b border-border-subtle bg-background-primary"
          onPress={() => handleConversationPress(item.id)}
          onLongPress={() => handleConversationLongPress(item)}
          delayLongPress={300}
        >
          <View className="flex-1">
            <Text className="text-base font-medium text-text-primary mb-0.5" numberOfLines={1}>
              {item.title || 'Untitled'}
            </Text>
            <Text className="text-sm text-text-tertiary">{formatRelativeDate(item.updated_at)}</Text>
          </View>
          <Text className="text-xl text-text-tertiary ml-2">›</Text>
        </TouchableOpacity>
      </SwipeableRow>
    );
  };

  return (
    <SafeAreaView className="flex-1 bg-background-primary">
      {/* Header */}
      <View className="flex-row items-center px-3 py-2 border-b border-border-subtle">
        <TouchableOpacity
          className="w-10 h-10 items-center justify-center"
          onPress={() => navigation.goBack()}
          testID="back-button"
        >
          <Feather name="arrow-left" size={24} color={colors.text.primary} />
        </TouchableOpacity>
        <Text className="flex-1 text-lg font-semibold text-text-primary text-center">Conversations</Text>
        <View className="w-10" />
      </View>

      {/* Error Display */}
      {error && (
        <View className="mx-3 mt-2 p-3 bg-error/10 border border-error/30 rounded-lg flex-row items-center justify-between">
          <Text className="flex-1 text-error text-sm mr-3">{error}</Text>
          <TouchableOpacity
            className="px-3 py-1.5 bg-error/20 rounded-md"
            onPress={() => {
              setError(null);
              loadConversations();
            }}
          >
            <Text className="text-error text-sm font-semibold">Retry</Text>
          </TouchableOpacity>
        </View>
      )}

      {/* Conversations List */}
      {isLoading ? (
        <View className="flex-1 items-center justify-center">
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      ) : (
        <FlatList
          data={filteredConversations}
          renderItem={renderConversation}
          keyExtractor={(item) => item.id}
          contentContainerStyle={{ flexGrow: 1, paddingBottom: 80 }}
          showsVerticalScrollIndicator={false}
          ListEmptyComponent={
            <View className="flex-1 items-center justify-center pt-16">
              {/* Icon with subtle glow */}
              <View
                className="w-20 h-20 rounded-full items-center justify-center mb-4"
                style={{
                  backgroundColor: 'rgba(139, 92, 246, 0.1)',
                  shadowColor: colors.pierre.violet,
                  shadowOffset: { width: 0, height: 0 },
                  shadowOpacity: 0.3,
                  shadowRadius: 20,
                }}
              >
                <Feather
                  name={searchQuery ? 'search' : 'message-circle'}
                  size={36}
                  color={colors.pierre.violet}
                />
              </View>
              <Text className="text-lg font-semibold text-text-primary mb-1">
                {searchQuery ? 'No Results' : 'No Conversations'}
              </Text>
              <Text className="text-base text-text-secondary text-center px-6">
                {searchQuery ? 'Try a different search term' : 'Start a conversation with your AI coach'}
              </Text>
            </View>
          }
        />
      )}

      {/* Floating Bottom Bar with Search and New Chat */}
      <View
        className="absolute left-3 right-3 flex-row items-center gap-3"
        style={{ bottom: spacing.lg }}
      >
        <View
          className="flex-1 flex-row items-center px-4 py-2"
          style={[{ height: 48 }, searchBarStyle]}
        >
          <Feather name="search" size={18} color={colors.text.tertiary} />
          <TextInput
            className="flex-1 text-base text-text-primary ml-3"
            placeholder="Search conversations"
            placeholderTextColor={colors.text.tertiary}
            value={searchQuery}
            onChangeText={setSearchQuery}
          />
        </View>
        <TouchableOpacity
          className="w-12 h-12 rounded-full items-center justify-center"
          style={fabStyle}
          onPress={handleNewChat}
        >
          <Feather name="plus" size={24} color="#FFFFFF" />
        </TouchableOpacity>
      </View>

      {/* Action Menu Modal */}
      <Modal
        visible={actionMenuVisible}
        animationType="fade"
        transparent
        onRequestClose={closeActionMenu}
      >
        <TouchableOpacity
          className="flex-1 bg-black/50 justify-center items-center"
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View
            className="min-w-[240px] overflow-hidden"
            style={menuStyle}
          >
            {/* Gradient accent bar */}
            <LinearGradient
              colors={gradients.violetCyan as [string, string]}
              start={{ x: 0, y: 0 }}
              end={{ x: 1, y: 0 }}
              style={{ height: 3, width: '100%' }}
            />
            <View className="py-2">
              <TouchableOpacity
                className="flex-row items-center px-4 py-3 opacity-40"
                disabled
              >
                <Feather name="star" size={18} color={colors.text.tertiary} />
                <Text className="text-base text-text-tertiary ml-3">Add to favorites</Text>
              </TouchableOpacity>

              <TouchableOpacity className="flex-row items-center px-4 py-3" onPress={handleRename}>
                <Feather name="edit-2" size={18} color={colors.text.primary} />
                <Text className="text-base text-text-primary ml-3">Rename</Text>
              </TouchableOpacity>

              <TouchableOpacity className="flex-row items-center px-4 py-3" onPress={handleDelete}>
                <Feather name="trash-2" size={18} color={colors.error} />
                <Text className="text-base text-error ml-3">Delete</Text>
              </TouchableOpacity>
            </View>
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
    </SafeAreaView>
  );
}
