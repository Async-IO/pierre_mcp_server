// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Type-safe translation hook wrapper for better developer experience
// ABOUTME: Provides autocomplete and type checking for translation keys

import { useTranslation as useI18nextTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';

// Define the structure of translation keys based on the JSON files
export interface TranslationKeys {
  common: {
    appName: string;
    welcome: string;
    email: string;
    password: string;
    login: string;
    logout: string;
    register: string;
    cancel: string;
    save: string;
    delete: string;
    edit: string;
    confirm: string;
    back: string;
    next: string;
    loading: string;
    error: string;
    success: string;
    search: string;
    filter: string;
    settings: string;
    profile: string;
    notifications: string;
    language: string;
    theme: string;
    help: string;
    about: string;
    close: string;
    yes: string;
    no: string;
    submit: string;
    retry: string;
  };
  auth: {
    signInWithGoogle: string;
    signInWithEmail: string;
    emailRequired: string;
    passwordRequired: string;
    invalidEmail: string;
    invalidCredentials: string;
    loginFailed: string;
    logoutSuccess: string;
    createAccount: string;
    alreadyHaveAccount: string;
    forgotPassword: string;
    rememberMe: string;
    confirmPassword: string;
    passwordMismatch: string;
    registrationSuccess: string;
    registrationFailed: string;
    pendingApproval: string;
    pendingApprovalMessage: string;
  };
  chat: Record<string, string>;
  coaches: Record<string, string>;
  settings: Record<string, string>;
  social: Record<string, string>;
  insights: Record<string, string>;
  providers: Record<string, string>;
  errors: Record<string, string>;
  validation: Record<string, string>;
}

/**
 * Type-safe translation hook
 * Usage: const { t } = useTranslation();
 * Then: t('common.welcome') or t('auth.loginFailed')
 */
export function useTranslation() {
  const { t, i18n } = useI18nextTranslation();
  
  return {
    t: t as TFunction,
    i18n,
    language: i18n.language,
    changeLanguage: i18n.changeLanguage.bind(i18n),
  };
}

export type { TFunction };
