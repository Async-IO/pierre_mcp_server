// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Language switcher hook for managing language changes
// ABOUTME: Provides language persistence across app sessions

import { useEffect, useCallback } from 'react';
import { useTranslation } from './types';
import type { SupportedLanguage } from './config';

export interface LanguageSwitcherOptions {
  storageKey?: string;
  onLanguageChange?: (language: SupportedLanguage) => void;
}

/**
 * Hook for managing language switching with persistence
 * @param options Configuration options for the language switcher
 */
export function useLanguageSwitcher(options: LanguageSwitcherOptions = {}) {
  const { storageKey = 'pierre_app_language', onLanguageChange } = options;
  const { i18n, language } = useTranslation();

  // Load saved language on mount
  useEffect(() => {
    const savedLanguage = localStorage.getItem(storageKey);
    if (savedLanguage && savedLanguage !== language) {
      i18n.changeLanguage(savedLanguage);
    }
  }, [storageKey, i18n, language]);

  // Change language and persist
  const changeLanguage = useCallback(
    (newLanguage: SupportedLanguage) => {
      i18n.changeLanguage(newLanguage);
      localStorage.setItem(storageKey, newLanguage);
      onLanguageChange?.(newLanguage);
    },
    [i18n, storageKey, onLanguageChange]
  );

  return {
    currentLanguage: language as SupportedLanguage,
    changeLanguage,
  };
}
