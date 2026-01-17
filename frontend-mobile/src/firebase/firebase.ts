// ABOUTME: Firebase SDK configuration and authentication utilities for React Native
// ABOUTME: Provides Google Sign-In via Firebase and expo-auth-session for the Pierre mobile app

import { initializeApp, type FirebaseApp } from 'firebase/app';
import {
  getAuth,
  onAuthStateChanged,
  signOut,
  GoogleAuthProvider,
  signInWithCredential,
  type Auth,
  type User,
} from 'firebase/auth';
import * as Google from 'expo-auth-session/providers/google';
import * as WebBrowser from 'expo-web-browser';
import type { AuthSessionResult } from 'expo-auth-session';
import { Platform } from 'react-native';

// Complete any pending auth sessions on app load
WebBrowser.maybeCompleteAuthSession();

// Firebase configuration - all values from environment variables
// Set these in your .env file with EXPO_PUBLIC_ prefix
const firebaseConfig = {
  apiKey: process.env.EXPO_PUBLIC_FIREBASE_API_KEY,
  authDomain: process.env.EXPO_PUBLIC_FIREBASE_AUTH_DOMAIN,
  projectId: process.env.EXPO_PUBLIC_FIREBASE_PROJECT_ID,
  storageBucket: process.env.EXPO_PUBLIC_FIREBASE_STORAGE_BUCKET,
  messagingSenderId: process.env.EXPO_PUBLIC_FIREBASE_MESSAGING_SENDER_ID,
  appId: process.env.EXPO_PUBLIC_FIREBASE_APP_ID,
};

// Google OAuth client IDs - needed for expo-auth-session
const googleClientIds = {
  iosClientId: process.env.EXPO_PUBLIC_GOOGLE_IOS_CLIENT_ID,
  androidClientId: process.env.EXPO_PUBLIC_GOOGLE_ANDROID_CLIENT_ID,
  webClientId: process.env.EXPO_PUBLIC_GOOGLE_WEB_CLIENT_ID,
};

// Check if Firebase is configured
const isFirebaseConfigured = Boolean(
  firebaseConfig.apiKey &&
  firebaseConfig.authDomain &&
  firebaseConfig.projectId
);

// Check if Google OAuth is configured for the current platform
// iOS requires iosClientId, Android requires androidClientId, web uses webClientId
function isPlatformGoogleOAuthConfigured(): boolean {
  if (Platform.OS === 'ios') {
    return Boolean(googleClientIds.iosClientId);
  }
  if (Platform.OS === 'android') {
    return Boolean(googleClientIds.androidClientId);
  }
  // Web/other platforms can use webClientId
  return Boolean(googleClientIds.webClientId);
}

let app: FirebaseApp | null = null;
let auth: Auth | null = null;

/**
 * Check if Firebase is properly configured via environment variables
 * Returns true only if both Firebase and platform-specific Google OAuth are configured
 */
export function isFirebaseEnabled(): boolean {
  return isFirebaseConfigured && isPlatformGoogleOAuthConfigured();
}

/**
 * Initialize Firebase app (lazy initialization)
 * Returns null if Firebase is not configured
 */
export function getFirebaseApp(): FirebaseApp | null {
  if (!isFirebaseConfigured) {
    return null;
  }
  if (!app) {
    app = initializeApp(firebaseConfig);
  }
  return app;
}

/**
 * Get Firebase Auth instance
 * Returns null if Firebase is not configured
 */
export function getFirebaseAuth(): Auth | null {
  if (!isFirebaseConfigured) {
    return null;
  }
  if (!auth) {
    const firebaseApp = getFirebaseApp();
    if (!firebaseApp) {
      return null;
    }
    auth = getAuth(firebaseApp);
  }
  return auth;
}

/**
 * Hook to get Google auth request for expo-auth-session
 * This should be called at the top level of a component (unconditionally)
 * Returns null values when Firebase is not enabled
 */
export function useGoogleAuth() {
  // Always call the hook unconditionally (React Rules of Hooks requirement)
  // webClientId is used with the Expo auth proxy (https://auth.expo.io/@owner/slug)
  // iosClientId/androidClientId are used for native redirects in standalone builds
  const [request, response, promptAsync] = Google.useAuthRequest({
    iosClientId: googleClientIds.iosClientId,
    androidClientId: googleClientIds.androidClientId,
    webClientId: googleClientIds.webClientId,
    scopes: ['email', 'profile'],
  });

  // Return null-like values if Firebase is not enabled
  if (!isFirebaseEnabled()) {
    return { request: null, response: null, promptAsync: null };
  }

  return { request, response, promptAsync };
}

/**
 * Exchange Google auth response for Firebase credential and sign in
 * Returns the Firebase ID token for backend authentication
 */
export async function signInWithGoogleResponse(
  response: AuthSessionResult
): Promise<{ idToken: string; email: string; displayName: string | null } | null> {
  if (response.type !== 'success') {
    return null;
  }

  const firebaseAuth = getFirebaseAuth();
  if (!firebaseAuth) {
    throw new Error('Google Sign-In is not available. Firebase is not configured.');
  }

  const { id_token: googleIdToken } = response.params;
  if (!googleIdToken) {
    throw new Error('No ID token received from Google');
  }

  // Create Firebase credential from Google ID token
  const credential = GoogleAuthProvider.credential(googleIdToken);

  // Sign in to Firebase with the credential
  const userCredential = await signInWithCredential(firebaseAuth, credential);

  // Get Firebase ID token for backend authentication
  const firebaseIdToken = await userCredential.user.getIdToken();

  return {
    idToken: firebaseIdToken,
    email: userCredential.user.email || '',
    displayName: userCredential.user.displayName,
  };
}

/**
 * Sign out from Firebase
 * No-op if Firebase is not configured
 */
export async function signOutFromFirebase(): Promise<void> {
  const firebaseAuth = getFirebaseAuth();
  if (!firebaseAuth) {
    return;
  }
  await signOut(firebaseAuth);
}

/**
 * Subscribe to Firebase auth state changes
 * Returns an unsubscribe function (no-op if Firebase not configured)
 */
export function subscribeToAuthState(
  callback: (user: User | null) => void
): () => void {
  const firebaseAuth = getFirebaseAuth();
  if (!firebaseAuth) {
    return () => {};
  }
  return onAuthStateChanged(firebaseAuth, callback);
}

/**
 * Get the current Firebase user (if signed in)
 * Returns null if Firebase not configured
 */
export function getCurrentFirebaseUser(): User | null {
  const firebaseAuth = getFirebaseAuth();
  if (!firebaseAuth) {
    return null;
  }
  return firebaseAuth.currentUser;
}

/**
 * Get ID token for current user
 * Returns null if Firebase not configured or no user
 */
export async function getFirebaseIdToken(): Promise<string | null> {
  const user = getCurrentFirebaseUser();
  if (!user) {
    return null;
  }
  return user.getIdToken();
}

// Re-export auth session types for convenience
export type { AuthSessionResult } from 'expo-auth-session';
