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
import { colors, spacing } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import { PromptDialog } from '../../components/ui';
import type { Conversation } from '../../types';
import type { ChatStackParamList } from '../../navigation/MainTabs';

interface ConversationsScreenProps {
  navigation: NativeStackNavigationProp<ChatStackParamList>;
}

// Shadow styles (React Native shadows cannot use className)
const searchContainerShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.25,
  shadowRadius: 4,
  elevation: 4,
};

const fabShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.25,
  shadowRadius: 4,
  elevation: 4,
};

const menuShadow: ViewStyle = {
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 4 },
  shadowOpacity: 0.3,
  shadowRadius: 8,
  elevation: 8,
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

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoading(true);
      const response = await apiService.getConversations();
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
    } catch (error) {
      console.error('Failed to load conversations:', error);
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
      const updated = await apiService.updateConversation(selectedConversation.id, {
        title: newTitle,
      });
      setConversations((prev) =>
        prev.map((c) => (c.id === selectedConversation.id ? { ...c, title: updated.title } : c))
      );
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
              setConversations((prev) => prev.filter((c) => c.id !== selectedConversation.id));
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

  const renderConversation = ({ item }: { item: Conversation }) => (
    <TouchableOpacity
      className="flex-row items-center px-4 py-3 border-b border-border-subtle"
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
  );

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
              <Text className="text-base text-text-tertiary">
                {searchQuery ? 'No conversations found' : 'No conversations yet'}
              </Text>
            </View>
          }
        />
      )}

      {/* Floating Bottom Bar with Search and New Chat */}
      <View
        className="absolute left-3 right-3 flex-row items-center gap-2"
        style={{ bottom: spacing.lg }}
      >
        <View
          className="flex-1 flex-row items-center bg-background-secondary rounded-full px-3 py-1"
          style={[{ height: 44 }, searchContainerShadow]}
        >
          <Text className="text-base mr-2">🔍</Text>
          <TextInput
            className="flex-1 text-base text-text-primary"
            placeholder="Search"
            placeholderTextColor={colors.text.tertiary}
            value={searchQuery}
            onChangeText={setSearchQuery}
          />
        </View>
        <TouchableOpacity
          className="w-11 h-11 rounded-full bg-primary-500 items-center justify-center"
          style={fabShadow}
          onPress={handleNewChat}
        >
          <Text className="text-3xl text-text-primary" style={{ marginTop: -2 }}>+</Text>
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
          className="flex-1 bg-black/30 justify-center items-center"
          activeOpacity={1}
          onPress={closeActionMenu}
        >
          <View
            className="bg-background-primary rounded-lg py-1 min-w-[200px]"
            style={menuShadow}
          >
            <TouchableOpacity
              className="flex-row items-center px-3 py-2 opacity-40"
              disabled
            >
              <Text className="text-lg mr-2 w-6">☆</Text>
              <Text className="text-base text-text-tertiary">Add to favorites</Text>
            </TouchableOpacity>

            <TouchableOpacity className="flex-row items-center px-3 py-2" onPress={handleRename}>
              <Text className="text-lg mr-2 w-6">✎</Text>
              <Text className="text-base text-text-primary">Rename</Text>
            </TouchableOpacity>

            <TouchableOpacity className="flex-row items-center px-3 py-2" onPress={handleDelete}>
              <Text className="text-lg mr-2 w-6">🗑</Text>
              <Text className="text-base text-error">Delete</Text>
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
    </SafeAreaView>
  );
}
