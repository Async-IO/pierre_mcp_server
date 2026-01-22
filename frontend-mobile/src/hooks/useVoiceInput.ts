// ABOUTME: Custom hook for speech-to-text voice input functionality
// ABOUTME: Wraps @react-native-voice/voice with state management and error handling

import { useState, useEffect, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import Constants, { ExecutionEnvironment } from 'expo-constants';

// Conditionally import Voice - it's not available in Expo Go
// The native module only exists in development builds
const isExpoGo = Constants.executionEnvironment === ExecutionEnvironment.StoreClient;

// Dynamic import types for Voice module
type VoiceModule = typeof import('@react-native-voice/voice').default;
type SpeechResultsEvent = import('@react-native-voice/voice').SpeechResultsEvent;
type SpeechErrorEvent = import('@react-native-voice/voice').SpeechErrorEvent;
type SpeechStartEvent = import('@react-native-voice/voice').SpeechStartEvent;
type SpeechEndEvent = import('@react-native-voice/voice').SpeechEndEvent;

// Only import Voice in development builds, not Expo Go
let Voice: VoiceModule | null = null;
if (!isExpoGo) {
  try {
    Voice = require('@react-native-voice/voice').default;
  } catch {
    // Voice module not available - running in Expo Go or native module not linked
    Voice = null;
  }
}

// Voice recognition error types for consumer handling
export type VoiceErrorType =
  | 'permission_denied'
  | 'no_speech'
  | 'network_error'
  | 'timeout'
  | 'not_available'
  | 'unknown';

export interface VoiceError {
  type: VoiceErrorType;
  message: string;
}

interface VoiceInputState {
  isListening: boolean;
  transcript: string;
  partialTranscript: string;
  error: VoiceError | null;
  isAvailable: boolean;
}

interface UseVoiceInputResult extends VoiceInputState {
  startListening: () => Promise<void>;
  stopListening: () => Promise<void>;
  cancelListening: () => Promise<void>;
  clearTranscript: () => void;
  clearError: () => void;
}

// Timeout duration for voice input (30 seconds)
const VOICE_TIMEOUT_MS = 30000;

// Parse Voice error codes into typed errors
function parseVoiceError(event: SpeechErrorEvent): VoiceError {
  const code = event.error?.code;
  const message = event.error?.message || 'Speech recognition failed';

  // Common error codes from @react-native-voice/voice
  // iOS: https://developer.apple.com/documentation/speech/sfspeechrecognitiontask
  // Android: https://developer.android.com/reference/android/speech/SpeechRecognizer
  switch (code) {
    case '5': // iOS: Access denied / Android: ERROR_CLIENT
    case 'recognition_fail':
      return { type: 'permission_denied', message: 'Microphone access denied' };
    case '7': // iOS: No match / Android: ERROR_NO_MATCH
    case 'no_match':
      return { type: 'no_speech', message: "Didn't catch that. Try again." };
    case '2': // Android: ERROR_NETWORK
    case '9': // Android: ERROR_INSUFFICIENT_PERMISSIONS
    case 'network':
      return { type: 'network_error', message: 'Network error. Please try again.' };
    case '6': // Android: ERROR_SPEECH_TIMEOUT
    case 'timeout':
      return { type: 'timeout', message: 'Voice input timed out' };
    default:
      return { type: 'unknown', message };
  }
}

export function useVoiceInput(): UseVoiceInputResult {
  const [state, setState] = useState<VoiceInputState>({
    isListening: false,
    transcript: '',
    partialTranscript: '',
    error: null,
    isAvailable: false,
  });

  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clear timeout helper
  const clearTimeoutRef = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    // If Voice module is not available (Expo Go), mark as unavailable and skip setup
    if (!Voice) {
      setState((prev) => ({
        ...prev,
        isAvailable: false,
      }));
      return;
    }

    // Check if voice recognition is available on this device
    Voice.isAvailable().then((available) => {
      // Voice.isAvailable() returns number (0/1) on some platforms, boolean on others
      const voiceAvailable = Boolean(available);
      setState((prev) => ({
        ...prev,
        isAvailable: voiceAvailable,
      }));
    });

    // Set up event listeners
    const onSpeechStart = (_event: SpeechStartEvent) => {
      setState((prev) => ({ ...prev, isListening: true, error: null }));
    };

    const onSpeechEnd = (_event: SpeechEndEvent) => {
      clearTimeoutRef();
      setState((prev) => {
        // Check if we got no transcript at all - that's a "no speech" error
        if (!prev.transcript && !prev.partialTranscript) {
          return {
            ...prev,
            isListening: false,
            error: { type: 'no_speech', message: "Didn't catch that. Try again." },
          };
        }
        return { ...prev, isListening: false };
      });
    };

    const onSpeechResults = (event: SpeechResultsEvent) => {
      clearTimeoutRef();
      const results = event.value;
      if (results && results.length > 0) {
        setState((prev) => ({
          ...prev,
          transcript: results[0],
          partialTranscript: '',
        }));
      }
    };

    const onSpeechPartialResults = (event: SpeechResultsEvent) => {
      const results = event.value;
      if (results && results.length > 0) {
        setState((prev) => ({ ...prev, partialTranscript: results[0] }));
      }
    };

    const onSpeechError = (event: SpeechErrorEvent) => {
      clearTimeoutRef();
      const voiceError = parseVoiceError(event);
      setState((prev) => ({
        ...prev,
        isListening: false,
        error: voiceError,
      }));
    };

    Voice.onSpeechStart = onSpeechStart;
    Voice.onSpeechEnd = onSpeechEnd;
    Voice.onSpeechResults = onSpeechResults;
    Voice.onSpeechPartialResults = onSpeechPartialResults;
    Voice.onSpeechError = onSpeechError;

    // Cleanup on unmount
    return () => {
      clearTimeoutRef();
      Voice.destroy().then(Voice.removeAllListeners);
    };
  }, [clearTimeoutRef]);

  const startListening = useCallback(async () => {
    // Check if Voice module is available (not in Expo Go)
    if (!Voice || !state.isAvailable) {
      setState((prev) => ({
        ...prev,
        error: { type: 'not_available', message: 'Speech recognition is not available on this device.' },
      }));
      return;
    }

    try {
      clearTimeoutRef();
      setState((prev) => ({
        ...prev,
        transcript: '',
        partialTranscript: '',
        error: null,
      }));

      // Use device locale, defaulting to en-US
      const locale = Platform.OS === 'ios' ? 'en-US' : 'en-US';
      await Voice.start(locale);

      // Set up timeout to auto-stop after VOICE_TIMEOUT_MS
      timeoutRef.current = setTimeout(async () => {
        try {
          await Voice.stop();
          setState((prev) => ({
            ...prev,
            isListening: false,
            error: { type: 'timeout', message: 'Voice input timed out. Try again.' },
          }));
        } catch {
          // Ignore stop errors during timeout
        }
      }, VOICE_TIMEOUT_MS);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : 'Failed to start voice recognition';
      // Check for permission-related errors
      const isPermissionError =
        errorMessage.toLowerCase().includes('permission') ||
        errorMessage.toLowerCase().includes('denied') ||
        errorMessage.toLowerCase().includes('not authorized');
      setState((prev) => ({
        ...prev,
        error: {
          type: isPermissionError ? 'permission_denied' : 'unknown',
          message: errorMessage,
        },
      }));
    }
  }, [state.isAvailable, clearTimeoutRef]);

  const stopListening = useCallback(async () => {
    clearTimeoutRef();
    if (!Voice) return;
    try {
      await Voice.stop();
    } catch (error) {
      console.error('Failed to stop voice recognition:', error);
    }
  }, [clearTimeoutRef]);

  const cancelListening = useCallback(async () => {
    clearTimeoutRef();
    if (!Voice) return;
    try {
      await Voice.cancel();
      setState((prev) => ({
        ...prev,
        isListening: false,
        partialTranscript: '',
      }));
    } catch (error) {
      console.error('Failed to cancel voice recognition:', error);
    }
  }, [clearTimeoutRef]);

  const clearTranscript = useCallback(() => {
    setState((prev) => ({
      ...prev,
      transcript: '',
      partialTranscript: '',
    }));
  }, []);

  const clearError = useCallback(() => {
    setState((prev) => ({
      ...prev,
      error: null,
    }));
  }, []);

  return {
    ...state,
    startListening,
    stopListening,
    cancelListening,
    clearTranscript,
    clearError,
  };
}
