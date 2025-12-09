// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Firebase SDK configuration and authentication utilities
// ABOUTME: Provides Google Sign-In via Firebase for the Pierre frontend

import { initializeApp, type FirebaseApp } from 'firebase/app';
import {
  getAuth,
  signInWithRedirect,
  getRedirectResult,
  onAuthStateChanged,
  GoogleAuthProvider,
  signOut,
  type Auth,
  type UserCredential,
  type User,
} from 'firebase/auth';

// Firebase configuration for Pierre Fitness Intelligence
const firebaseConfig = {
  apiKey: import.meta.env.VITE_FIREBASE_API_KEY || 'AIzaSyAYYmGwtoZK1xWdZqkrKHQTgsw6I3ExZjY',
  authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN || 'pierre-fitness-intelligence.firebaseapp.com',
  projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID || 'pierre-fitness-intelligence',
  storageBucket: 'pierre-fitness-intelligence.firebasestorage.app',
  messagingSenderId: '779931405774',
  appId: '1:779931405774:web:949695e2beb6e3f5da6f9f',
};

let app: FirebaseApp | null = null;
let auth: Auth | null = null;

/**
 * Initialize Firebase app (lazy initialization)
 */
export function getFirebaseApp(): FirebaseApp {
  if (!app) {
    app = initializeApp(firebaseConfig);
  }
  return app;
}

/**
 * Get Firebase Auth instance
 */
export function getFirebaseAuth(): Auth {
  if (!auth) {
    auth = getAuth(getFirebaseApp());
  }
  return auth;
}

/**
 * Initiate Google sign-in via redirect flow
 * After calling this, the page will redirect to Google's login page.
 * On return, call checkGoogleRedirectResult() to get the authentication result.
 */
export async function signInWithGoogle(): Promise<void> {
  const firebaseAuth = getFirebaseAuth();

  const provider = new GoogleAuthProvider();
  provider.addScope('email');
  provider.addScope('profile');

  await signInWithRedirect(firebaseAuth, provider);
}

/**
 * Check for Google sign-in redirect result on page load
 * Returns null if no redirect result is pending, otherwise returns user data
 */
export async function checkGoogleRedirectResult(): Promise<{
  idToken: string;
  email: string;
  displayName: string | null;
} | null> {
  const firebaseAuth = getFirebaseAuth();

  const result: UserCredential | null = await getRedirectResult(firebaseAuth);
  if (!result) {
    return null;
  }

  const idToken = await result.user.getIdToken();

  return {
    idToken,
    email: result.user.email || '',
    displayName: result.user.displayName,
  };
}

/**
 * Sign out from Firebase
 */
export async function signOutFromFirebase(): Promise<void> {
  const firebaseAuth = getFirebaseAuth();
  await signOut(firebaseAuth);
}

/**
 * Subscribe to Firebase auth state changes
 * Returns an unsubscribe function
 */
export function subscribeToAuthState(
  callback: (user: User | null) => void
): () => void {
  const firebaseAuth = getFirebaseAuth();
  return onAuthStateChanged(firebaseAuth, callback);
}

/**
 * Get the current Firebase user (if signed in)
 */
export function getCurrentFirebaseUser(): User | null {
  const firebaseAuth = getFirebaseAuth();
  return firebaseAuth.currentUser;
}

/**
 * Get ID token for current user
 */
export async function getFirebaseIdToken(): Promise<string | null> {
  const user = getCurrentFirebaseUser();
  if (!user) {
    return null;
  }
  return user.getIdToken();
}
