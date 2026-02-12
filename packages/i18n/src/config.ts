// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: i18next configuration for unified web and mobile internationalization
// ABOUTME: Configures language detection, resource loading, and fallback behavior

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import enTranslation from './locales/en/translation.json';
import esTranslation from './locales/es/translation.json';
import frTranslation from './locales/fr/translation.json';

export const SUPPORTED_LANGUAGES = ['en', 'es', 'fr'] as const;
export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number];

export const LANGUAGE_NAMES: Record<SupportedLanguage, string> = {
  en: 'English',
  es: 'Español',
  fr: 'Français',
};

export const DEFAULT_LANGUAGE: SupportedLanguage = 'en';

// Default i18n configuration
export const defaultI18nConfig = {
  resources: {
    en: { translation: enTranslation },
    es: { translation: esTranslation },
    fr: { translation: frTranslation },
  },
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  interpolation: {
    escapeValue: false, // React already escapes values
  },
  react: {
    useSuspense: false, // Set to true if you want to use Suspense
  },
};

/**
 * Initialize i18next for web applications
 * @param config Optional configuration to override defaults
 */
export function initI18n(config?: Partial<typeof defaultI18nConfig>) {
  return i18n
    .use(initReactI18next)
    .init({
      ...defaultI18nConfig,
      ...config,
    });
}

export { i18n };
