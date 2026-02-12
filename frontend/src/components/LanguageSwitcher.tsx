// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Language switcher dropdown component for web frontend
// ABOUTME: Provides user-friendly language selection with flags and names

import { useLanguageSwitcher, SUPPORTED_LANGUAGES, LANGUAGE_NAMES, type SupportedLanguage } from '@pierre/i18n';

const LANGUAGE_FLAGS: Record<SupportedLanguage, string> = {
  en: '🇺🇸',
  es: '🇪🇸',
  fr: '🇫🇷',
};

export function LanguageSwitcher() {
  const { currentLanguage, changeLanguage } = useLanguageSwitcher();

  return (
    <div className="relative inline-block">
      <select
        value={currentLanguage}
        onChange={(e) => changeLanguage(e.target.value as SupportedLanguage)}
        className="appearance-none bg-pierre-gray-800 text-white px-4 py-2 pr-10 rounded-lg border border-pierre-gray-700 hover:border-pierre-violet focus:outline-none focus:border-pierre-violet transition-colors cursor-pointer"
        aria-label="Select language"
      >
        {SUPPORTED_LANGUAGES.map((lang) => (
          <option key={lang} value={lang}>
            {LANGUAGE_FLAGS[lang]} {LANGUAGE_NAMES[lang]}
          </option>
        ))}
      </select>
      <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-pierre-gray-400">
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </div>
    </div>
  );
}
