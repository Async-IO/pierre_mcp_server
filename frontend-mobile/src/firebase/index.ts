// ABOUTME: Barrel export for Firebase module
// ABOUTME: Re-exports all Firebase utilities for convenient imports

export {
  isFirebaseEnabled,
  getFirebaseApp,
  getFirebaseAuth,
  useGoogleAuth,
  signInWithGoogleResponse,
  signOutFromFirebase,
  subscribeToAuthState,
  getCurrentFirebaseUser,
  getFirebaseIdToken,
  type AuthSessionResult,
} from './firebase';
