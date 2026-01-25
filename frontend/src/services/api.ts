// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API service re-export for backward compatibility
// ABOUTME: New code should import from './api/index' or specific domain modules

// Re-export everything from the modular API structure
// Note: Using explicit './api/index' to avoid ambiguity with this file
export * from './api/index';
export { apiService as default } from './api/index';
