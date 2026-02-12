// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Language switcher component for React Native mobile app
// ABOUTME: Provides touch-friendly language selection with visual feedback

import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { useLanguageSwitcherNative, SUPPORTED_LANGUAGES, LANGUAGE_NAMES, type SupportedLanguage } from '@pierre/i18n';

const LANGUAGE_FLAGS: Record<SupportedLanguage, string> = {
  en: '🇺🇸',
  es: '🇪🇸',
  fr: '🇫🇷',
};

export function LanguageSwitcher() {
  const { currentLanguage, changeLanguage } = useLanguageSwitcherNative();

  return (
    <View className="flex-row gap-3 p-4">
      {SUPPORTED_LANGUAGES.map((lang) => {
        const isSelected = currentLanguage === lang;
        return (
          <TouchableOpacity
            key={lang}
            onPress={() => changeLanguage(lang)}
            className={`flex-1 py-3 px-4 rounded-xl items-center ${
              isSelected
                ? 'bg-pierre-violet'
                : 'bg-pierre-gray-800 border border-pierre-gray-700'
            }`}
            activeOpacity={0.7}
          >
            <Text className="text-2xl mb-1">{LANGUAGE_FLAGS[lang]}</Text>
            <Text
              className={`text-sm font-medium ${
                isSelected ? 'text-white' : 'text-pierre-gray-300'
              }`}
            >
              {LANGUAGE_NAMES[lang]}
            </Text>
          </TouchableOpacity>
        );
      })}
    </View>
  );
}
