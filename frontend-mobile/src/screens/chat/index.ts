// ABOUTME: Barrel export for chat screen components and hooks
// ABOUTME: Provides convenient imports for chat screen module

// Main screen component
export { ChatScreen } from './ChatScreen';

// Components
export { ChatHeader } from './ChatHeader';
export { ChatInputBar } from './ChatInputBar';
export { MessageList } from './MessageList';
export { ProviderModal } from './ProviderModal';

// Hooks
export { useConversations } from './useConversations';
export type { ConversationsState, ConversationsActions } from './useConversations';

export { useMessages } from './useMessages';
export type { MessagesState, MessagesActions } from './useMessages';

export { useProviderStatus } from './useProviderStatus';
export type { ProviderStatusState, ProviderStatusActions } from './useProviderStatus';

export { useCoachSelection } from './useCoachSelection';
export type { CoachSelectionState, CoachSelectionActions } from './useCoachSelection';

export { useChatVoiceInput } from './useChatVoiceInput';
export type { ChatVoiceInputState, ChatVoiceInputActions } from './useChatVoiceInput';
