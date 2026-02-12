// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Main export file for @pierre/i18n package
// ABOUTME: Provides unified API for web and mobile i18n functionality

export { initI18n, i18n, defaultI18nConfig, SUPPORTED_LANGUAGES, LANGUAGE_NAMES, DEFAULT_LANGUAGE } from './config';
export type { SupportedLanguage } from './config';
export { useTranslation } from './types';
export type { TranslationKeys, TFunction } from './types';
export { useLanguageSwitcher } from './useLanguageSwitcher';
export { useLanguageSwitcherNative } from './useLanguageSwitcherNative';

// Re-export core i18next types for convenience
export type { i18n as I18nInstance, TOptions } from 'i18next';
export { I18nextProvider } from 'react-i18next';
