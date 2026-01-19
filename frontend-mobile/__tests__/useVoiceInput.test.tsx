// ABOUTME: Unit tests for useVoiceInput hook
// ABOUTME: Tests voice recognition state management and error handling

import { renderHook, act, waitFor } from '@testing-library/react-native';
import Voice from '@react-native-voice/voice';
import { useVoiceInput } from '../src/hooks/useVoiceInput';

// Voice is mocked in jest.setup.js

describe('useVoiceInput Hook', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
    // Reset mock implementations
    (Voice.isAvailable as jest.Mock).mockResolvedValue(1);
    (Voice.start as jest.Mock).mockResolvedValue(undefined);
    (Voice.stop as jest.Mock).mockResolvedValue(undefined);
    (Voice.cancel as jest.Mock).mockResolvedValue(undefined);
    (Voice.destroy as jest.Mock).mockResolvedValue(undefined);
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
        expect(Voice.isAvailable).toHaveBeenCalled();
      });
    });

    it('should set isAvailable true when voice is available', async () => {
      (Voice.isAvailable as jest.Mock).mockResolvedValue(1);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });
    });

    it('should set isAvailable false when voice is not available', async () => {
      (Voice.isAvailable as jest.Mock).mockResolvedValue(0);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(false);
      });
    });
  });

  describe('startListening', () => {
    it('should call Voice.start when available', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(Voice.start).toHaveBeenCalledWith('en-US');
    });

    it('should set error when voice not available', async () => {
      (Voice.isAvailable as jest.Mock).mockResolvedValue(0);
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(false);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.error?.type).toBe('not_available');
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

    it('should set error on start failure', async () => {
      (Voice.start as jest.Mock).mockRejectedValue(new Error('Permission denied'));
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.startListening();
      });

      expect(result.current.error?.type).toBe('permission_denied');
    });
  });

  describe('stopListening', () => {
    it('should call Voice.stop', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.stopListening();
      });

      expect(Voice.stop).toHaveBeenCalled();
    });
  });

  describe('cancelListening', () => {
    it('should call Voice.cancel', async () => {
      const { result } = renderHook(() => useVoiceInput());

      await waitFor(() => {
        expect(result.current.isAvailable).toBe(true);
      });

      await act(async () => {
        await result.current.cancelListening();
      });

      expect(Voice.cancel).toHaveBeenCalled();
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
      (Voice.isAvailable as jest.Mock).mockResolvedValue(0);
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

  describe('cleanup', () => {
    it('should call Voice.destroy on unmount', async () => {
      const { unmount } = renderHook(() => useVoiceInput());

      unmount();

      await waitFor(() => {
        expect(Voice.destroy).toHaveBeenCalled();
      });
    });
  });
});
