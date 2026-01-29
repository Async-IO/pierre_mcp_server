// ABOUTME: Custom hook for speech-to-text voice input functionality
// ABOUTME: Wraps expo-speech-recognition with state management and error handling

import { useState, useEffect, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import Constants, { ExecutionEnvironment } from 'expo-constants';
import {
  ExpoSpeechRecognitionModule,
  useSpeechRecognitionEvent,
} from 'expo-speech-recognition';
import type {
  ExpoSpeechRecognitionErrorCode,
  ExpoSpeechRecognitionErrorEvent,
  ExpoSpeechRecognitionResultEvent,
} from 'expo-speech-recognition';

// Check if running in Expo Go (native module won't be available)
const isExpoGo = Constants.executionEnvironment === ExecutionEnvironment.StoreClient;

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

// Map expo-speech-recognition error codes to our typed errors
function mapErrorCode(code: ExpoSpeechRecognitionErrorCode, message: string): VoiceError {
  switch (code) {
    case 'not-allowed':
      return { type: 'permission_denied', message: 'Microphone access denied' };
    case 'no-speech':
    case 'speech-timeout':
      return { type: 'no_speech', message: "Didn't catch that. Try again." };
    case 'network':
      return { type: 'network_error', message: 'Network error. Please try again.' };
    case 'service-not-allowed':
      return { type: 'not_available', message: 'Speech recognition is not available.' };
    case 'aborted':
      return { type: 'timeout', message: 'Voice input was cancelled.' };
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
    isAvailable: !isExpoGo, // Assume available if not in Expo Go; will verify on mount
  });

  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clear timeout helper
  const clearTimeoutRef = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  // Check availability on mount
  useEffect(() => {
    if (isExpoGo) {
      setState((prev) => ({
        ...prev,
        isAvailable: false,
      }));
      return;
    }

    // Check if speech recognition is available (synchronous call)
    const available = ExpoSpeechRecognitionModule.isRecognitionAvailable();
    setState((prev) => ({
      ...prev,
      isAvailable: available,
    }));
  }, []);

  // Handle speech start event
  useSpeechRecognitionEvent('start', () => {
    setState((prev) => ({ ...prev, isListening: true, error: null }));
  });

  // Handle speech end event
  useSpeechRecognitionEvent('end', () => {
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
  });

  // Handle speech results
  useSpeechRecognitionEvent('result', (event: ExpoSpeechRecognitionResultEvent) => {
    const results = event.results;
    if (results && results.length > 0) {
      const transcript = results[0].transcript;
      if (event.isFinal) {
        clearTimeoutRef();
        setState((prev) => ({
          ...prev,
          transcript,
          partialTranscript: '',
        }));
      } else {
        setState((prev) => ({ ...prev, partialTranscript: transcript }));
      }
    }
  });

  // Handle errors
  useSpeechRecognitionEvent('error', (event: ExpoSpeechRecognitionErrorEvent) => {
    clearTimeoutRef();
    const voiceError = mapErrorCode(event.error, event.message);
    setState((prev) => ({
      ...prev,
      isListening: false,
      error: voiceError,
    }));
  });

  const startListening = useCallback(async () => {
    // Check if running in Expo Go
    if (isExpoGo || !state.isAvailable) {
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

      // Request permissions first
      const permissionResult = await ExpoSpeechRecognitionModule.requestPermissionsAsync();
      if (!permissionResult.granted) {
        setState((prev) => ({
          ...prev,
          error: { type: 'permission_denied', message: 'Microphone permission denied.' },
        }));
        return;
      }

      // Use device locale, defaulting to en-US
      const locale = Platform.OS === 'ios' ? 'en-US' : 'en-US';

      // Start recognition with options
      ExpoSpeechRecognitionModule.start({
        lang: locale,
        interimResults: true,
        maxAlternatives: 1,
        continuous: false, // Stop after first utterance
      });

      // Set up timeout to auto-stop after VOICE_TIMEOUT_MS
      timeoutRef.current = setTimeout(() => {
        ExpoSpeechRecognitionModule.stop();
        setState((prev) => ({
          ...prev,
          isListening: false,
          error: { type: 'timeout', message: 'Voice input timed out. Try again.' },
        }));
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
    if (isExpoGo) return;
    try {
      ExpoSpeechRecognitionModule.stop();
    } catch (error) {
      console.error('Failed to stop voice recognition:', error);
    }
  }, [clearTimeoutRef]);

  const cancelListening = useCallback(async () => {
    clearTimeoutRef();
    if (isExpoGo) return;
    try {
      ExpoSpeechRecognitionModule.abort();
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
