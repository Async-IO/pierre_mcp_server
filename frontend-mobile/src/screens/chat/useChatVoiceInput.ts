// ABOUTME: Chat-specific wrapper for voice input with toast error handling
// ABOUTME: Integrates useVoiceInput hook with chat screen error notifications

import { useEffect, useCallback } from 'react';
import * as Linking from 'expo-linking';
import * as Haptics from 'expo-haptics';
import Toast from 'react-native-toast-message';
import { useVoiceInput as useBaseVoiceInput } from '../../hooks/useVoiceInput';
import type { VoiceError } from '../../hooks/useVoiceInput';

export interface ChatVoiceInputState {
  isListening: boolean;
  transcript: string;
  partialTranscript: string;
  error: VoiceError | null;
  isAvailable: boolean;
}

export interface ChatVoiceInputActions {
  handleVoicePress: () => Promise<void>;
  clearTranscript: () => void;
}

export function useChatVoiceInput(
  onTranscript: (text: string) => void,
  setInputText: (text: string) => void
): ChatVoiceInputState & ChatVoiceInputActions {
  const {
    isListening,
    transcript,
    partialTranscript,
    error: voiceError,
    isAvailable: voiceAvailable,
    startListening,
    stopListening,
    clearTranscript,
    clearError: clearVoiceError,
  } = useBaseVoiceInput();

  // Handle voice input transcript - replace input text with final transcript
  useEffect(() => {
    if (transcript) {
      onTranscript(transcript);
      clearTranscript();
    }
  }, [transcript, clearTranscript, onTranscript]);

  // Handle voice input errors - show toast notifications
  useEffect(() => {
    if (voiceError) {
      const showVoiceErrorToast = (error: VoiceError) => {
        if (error.type === 'permission_denied') {
          Toast.show({
            type: 'voiceError',
            text1: 'Microphone Access Required',
            text2: error.message,
            visibilityTime: 5000,
            props: {
              onOpenSettings: () => {
                Linking.openSettings();
                clearVoiceError();
              },
            },
          });
        } else if (error.type === 'no_speech') {
          Toast.show({
            type: 'voiceError',
            text1: 'No Speech Detected',
            text2: error.message,
            visibilityTime: 3000,
            props: {
              onRetry: () => {
                clearVoiceError();
                startListening();
              },
            },
          });
        } else if (error.type === 'network_error') {
          Toast.show({
            type: 'voiceError',
            text1: 'Network Error',
            text2: error.message,
            visibilityTime: 4000,
            props: {
              onRetry: () => {
                clearVoiceError();
                startListening();
              },
            },
          });
        } else if (error.type === 'timeout') {
          Toast.show({
            type: 'info',
            text1: 'Voice Input Timeout',
            text2: error.message,
            visibilityTime: 3000,
          });
        } else {
          Toast.show({
            type: 'error',
            text1: 'Voice Input Error',
            text2: error.message,
            visibilityTime: 3000,
          });
        }
      };

      showVoiceErrorToast(voiceError);
      if (voiceError.type !== 'permission_denied' && voiceError.type !== 'no_speech' && voiceError.type !== 'network_error') {
        clearVoiceError();
      }
    }
  }, [voiceError, clearVoiceError, startListening]);

  const handleVoicePress = useCallback(async () => {
    await Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Medium);

    if (isListening) {
      await stopListening();
    } else {
      setInputText('');
      await startListening();
    }
  }, [isListening, stopListening, startListening, setInputText]);

  return {
    isListening,
    transcript,
    partialTranscript,
    error: voiceError,
    isAvailable: voiceAvailable,
    handleVoicePress,
    clearTranscript,
  };
}
