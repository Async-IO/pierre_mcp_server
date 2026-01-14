// ABOUTME: Full conversations list screen with search functionality
// ABOUTME: Shows all conversations with relative dates and search bar

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  SafeAreaView,
  FlatList,
  TouchableOpacity,
  TextInput,
  ActivityIndicator,
  Alert,
  Modal,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import { colors, spacing, fontSize, borderRadius } from '../../constants/theme';
import { apiService } from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import type { Conversation } from '../../types';
import type { AppDrawerParamList } from '../../navigation/AppDrawer';

interface ConversationsScreenProps {
  navigation: DrawerNavigationProp<AppDrawerParamList>;
}

export function ConversationsScreen({ navigation }: ConversationsScreenProps) {
  const { isAuthenticated } = useAuth();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [filteredConversations, setFilteredConversations] = useState<Conversation[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [actionMenuVisible, setActionMenuVisible] = useState(false);
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(null);

  const loadConversations = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      setIsLoading(true);
      const response = await apiService.getConversations();
      const seen = new Set<string>();
      const deduplicated = (response.conversations || []).filter((conv) => {
        if (seen.has(conv.id)) return false;
        seen.add(conv.id);
        return true;
      });
      const sorted = deduplicated.sort(
        (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
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
          setConversations((prev) =>
            prev.map((c) => (c.id === selectedConversation.id ? { ...c, title: updated.title } : c))
          );
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
      style={styles.conversationItem}
      onPress={() => handleConversationPress(item.id)}
      onLongPress={() => handleConversationLongPress(item)}
      delayLongPress={300}
    >
      <View style={styles.conversationContent}>
        <Text style={styles.conversationTitle} numberOfLines={1}>
          {item.title || 'Untitled'}
        </Text>
        <Text style={styles.conversationDate}>{formatRelativeDate(item.updated_at)}</Text>
      </View>
      <Text style={styles.chevron}>›</Text>
    </TouchableOpacity>
  );

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => navigation.openDrawer()}
        >
          <Text style={styles.menuIcon}>☰</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>Conversations</Text>
        <View style={styles.headerSpacer} />
      </View>

      {/* Conversations List */}
      {isLoading ? (
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color={colors.primary[500]} />
        </View>
      ) : (
        <FlatList
          data={filteredConversations}
          renderItem={renderConversation}
          keyExtractor={(item) => item.id}
          contentContainerStyle={styles.listContent}
          showsVerticalScrollIndicator={false}
          ListEmptyComponent={
            <View style={styles.emptyContainer}>
              <Text style={styles.emptyText}>
                {searchQuery ? 'No conversations found' : 'No conversations yet'}
              </Text>
            </View>
          }
        />
      )}

      {/* Floating Bottom Bar with Search and New Chat */}
      <View style={styles.floatingBottomBar}>
        <View style={styles.searchContainer}>
          <Text style={styles.searchIcon}>🔍</Text>
          <TextInput
            style={styles.searchInput}
            placeholder="Search"
            placeholderTextColor={colors.text.tertiary}
            value={searchQuery}
            onChangeText={setSearchQuery}
          />
        </View>
        <TouchableOpacity style={styles.newChatFab} onPress={handleNewChat}>
          <Text style={styles.newChatIcon}>+</Text>
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

            <TouchableOpacity style={styles.actionMenuItem} onPress={handleRename}>
              <Text style={styles.actionMenuIcon}>✎</Text>
              <Text style={styles.actionMenuText}>Rename</Text>
            </TouchableOpacity>

            <TouchableOpacity style={styles.actionMenuItem} onPress={handleDelete}>
              <Text style={styles.actionMenuIconDanger}>🗑</Text>
              <Text style={styles.actionMenuTextDanger}>Delete</Text>
            </TouchableOpacity>
          </View>
        </TouchableOpacity>
      </Modal>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background.primary,
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
  },
  headerSpacer: {
    width: 40,
  },
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  listContent: {
    flexGrow: 1,
    paddingBottom: 80, // Space for floating bottom bar
  },
  conversationItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  conversationContent: {
    flex: 1,
  },
  conversationTitle: {
    fontSize: fontSize.md,
    fontWeight: '500',
    color: colors.text.primary,
    marginBottom: 2,
  },
  conversationDate: {
    fontSize: fontSize.sm,
    color: colors.text.tertiary,
  },
  chevron: {
    fontSize: 20,
    color: colors.text.tertiary,
    marginLeft: spacing.sm,
  },
  emptyContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingTop: spacing.xxl,
  },
  emptyText: {
    fontSize: fontSize.md,
    color: colors.text.tertiary,
  },
  floatingBottomBar: {
    position: 'absolute',
    bottom: spacing.lg,
    left: spacing.md,
    right: spacing.md,
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
  },
  searchContainer: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.background.secondary,
    borderRadius: borderRadius.full,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
    height: 44,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 4,
  },
  searchIcon: {
    fontSize: 16,
    marginRight: spacing.sm,
  },
  searchInput: {
    flex: 1,
    fontSize: fontSize.md,
    color: colors.text.primary,
  },
  newChatFab: {
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
});
