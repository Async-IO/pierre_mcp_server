// ABOUTME: Unit tests for useVoiceInput hook
// ABOUTME: Tests voice recognition state management and error handling

import { renderHook, act, waitFor } from '@testing-library/react-native';
import { ExpoSpeechRecognitionModule } from 'expo-speech-recognition';
import { useVoiceInput } from '../src/hooks/useVoiceInput';

// expo-speech-recognition is mocked in jest.setup.js

describe('useVoiceInput Hook', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
    // Reset mock implementations
    (ExpoSpeechRecognitionModule.isRecognitionAvailable as jest.Mock).mockReturnValue(true);
    (ExpoSpeechRecognitionModule.requestPermissionsAsync as jest.Mock).mockResolvedValue({ granted: true });
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  describe('initial state', () => {
    it('should start with isListening false', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isListening).toBe(false);
      });
    });

    it('should start with empty transcript', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.transcript).toBe('');
      });
    });

    it('should start with empty partialTranscript', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.partialTranscript).toBe('');
      });
    });

    it('should start with no error', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.error).toBeNull();
      });
    });

    it('should check voice availability on mount', async () => {
      renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(ExpoSpeechRecognitionModule.isRecognitionAvailable).toHaveBeenCalled();
      });
    });

    it('should set isAvailable true when voice is available', async () => {
      (ExpoSpeechRecognitionModule.isRecognitionAvailable as jest.Mock).mockReturnValue(true);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });
    });

    it('should set isAvailable false when voice is not available', async () => {
      (ExpoSpeechRecognitionModule.isRecognitionAvailable as jest.Mock).mockReturnValue(false);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(false);
      });
    });
  });

  describe('startListening', () => {
    it('should call ExpoSpeechRecognitionModule.start when available', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(ExpoSpeechRecognitionModule.start).toHaveBeenCalledWith({
        lang: 'en-US',
        interimResults: true,
        maxAlternatives: 1,
        continuous: false,
      });
    });

    it('should request permissions before starting', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(ExpoSpeechRecognitionModule.requestPermissionsAsync).toHaveBeenCalled();
    });

    it('should set error when voice not available', async () => {
      (ExpoSpeechRecognitionModule.isRecognitionAvailable as jest.Mock).mockReturnValue(false);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(false);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.error?.type).toBe('not_available');
    });

    it('should set error when permission denied', async () => {
      (ExpoSpeechRecognitionModule.requestPermissionsAsync as jest.Mock).mockResolvedValue({ granted: false });
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.error?.type).toBe('permission_denied');
    });

    it('should clear previous transcript when starting', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.transcript).toBe('');
      expect(result.current.partialTranscript).toBe('');
    });
  });

  describe('stopListening', () => {
    it('should call ExpoSpeechRecognitionModule.stop', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.stopListening();
      });

      expect(ExpoSpeechRecognitionModule.stop).toHaveBeenCalled();
    });
  });

  describe('cancelListening', () => {
    it('should call ExpoSpeechRecognitionModule.abort', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.cancelListening();
      });

      expect(ExpoSpeechRecognitionModule.abort).toHaveBeenCalled();
    });
  });

  describe('clearTranscript', () => {
    it('should clear transcript and partialTranscript', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      act(() => {
        result.current.clearTranscript();
      });

      expect(result.current.transcript).toBe('');
      expect(result.current.partialTranscript).toBe('');
    });
  });

  describe('clearError', () => {
    it('should clear error state', async () => {
      (ExpoSpeechRecognitionModule.isRecognitionAvailable as jest.Mock).mockReturnValue(false);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(false);
      });

      // Trigger an error
      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.error).not.toBeNull();

      // Clear the error
      act(() => {
        result.current.clearError();
      });

      expect(result.current.error).toBeNull();
    });
  });
});
