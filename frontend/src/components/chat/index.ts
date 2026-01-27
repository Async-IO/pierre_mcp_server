// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Barrel exports for chat components
// ABOUTME: Centralizes exports for easy importing throughout the application

// Types and utilities
export * from './types';
export * from './utils';

// Components
export { default as ChatSidebar } from './ChatSidebar';
export { default as ConversationItem } from './ConversationItem';
export { default as MessageItem } from './MessageItem';
export { default as MessageList } from './MessageList';
export { default as MessageInput } from './MessageInput';
export { default as MyCoachCard } from './MyCoachCard';
export { default as CategoryFilterButton } from './CategoryFilterButton';
export { default as ProviderConnectionModal } from './ProviderConnectionModal';
export { default as CoachFormModal } from './CoachFormModal';
