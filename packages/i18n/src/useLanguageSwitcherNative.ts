// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: React Native specific language switcher hook with AsyncStorage
// ABOUTME: Provides persistent language storage for mobile apps

import { useEffect, useCallback } from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { useTranslation } from './types';
import type { SupportedLanguage } from './config';

export interface LanguageSwitcherOptions {
  storageKey?: string;
  onLanguageChange?: (language: SupportedLanguage) => void;
}

/**
 * Hook for managing language switching with AsyncStorage persistence
 * @param options Configuration options for the language switcher
 */
export function useLanguageSwitcherNative(options: LanguageSwitcherOptions = {}) {
  const { storageKey = 'pierre_app_language', onLanguageChange } = options;
  const { i18n, language } = useTranslation();

  // Load saved language on mount
  useEffect(() => {
    const loadLanguage = async () => {
      try {
        const savedLanguage = await AsyncStorage.getItem(storageKey);
        if (savedLanguage && savedLanguage !== language) {
          await i18n.changeLanguage(savedLanguage);
        }
      } catch (error) {
        console.error('Failed to load saved language:', error);
      }
    };

    loadLanguage();
  }, [storageKey, i18n, language]);

  // Change language and persist
  const changeLanguage = useCallback(
    async (newLanguage: SupportedLanguage) => {
      try {
        await i18n.changeLanguage(newLanguage);
        await AsyncStorage.setItem(storageKey, newLanguage);
        onLanguageChange?.(newLanguage);
      } catch (error) {
        console.error('Failed to save language:', error);
      }
    },
    [i18n, storageKey, onLanguageChange]
  );

  return {
    currentLanguage: language as SupportedLanguage,
    changeLanguage,
  };
}
