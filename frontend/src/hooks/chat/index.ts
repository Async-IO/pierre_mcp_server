// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Re-exports all chat-related custom hooks
// ABOUTME: Centralizes imports for ChatTab refactoring

export { useConversations } from './useConversations';
export { useCoachManagement } from './useCoachManagement';
export { useOAuthHandler } from './useOAuthHandler';
export { useMessageStreaming } from './useMessageStreaming';

// Export types
export type { Coach, CoachFormData } from './useCoachManagement';
