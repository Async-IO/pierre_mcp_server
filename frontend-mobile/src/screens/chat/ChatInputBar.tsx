// ABOUTME: Chat input bar component with text input, voice, and send buttons
// ABOUTME: Displays paperclip attachment, voice input, and send button with states

import React from 'react';
import { View, TextInput, TouchableOpacity, ActivityIndicator, Text } from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { colors, glassCard } from '../../constants/theme';
import { VoiceButton } from '../../components/ui';

interface ChatInputBarProps {
  inputText: string;
  partialTranscript: string;
  isListening: boolean;
  isSending: boolean;
  voiceAvailable: boolean;
  insetBottom: number;
  inputRef: React.RefObject<TextInput | null>;
  onChangeText: (text: string) => void;
  onVoicePress: () => void;
  onSendMessage: () => void;
}

export function ChatInputBar({
  inputText,
  partialTranscript,
  isListening,
  isSending,
  voiceAvailable,
  insetBottom,
  inputRef,
  onChangeText,
  onVoicePress,
  onSendMessage,
}: ChatInputBarProps) {
  const displayText = isListening ? partialTranscript : inputText;
  const canSend = inputText.trim() && !isSending && !isListening;

  return (
    <View className="px-4 py-2" style={{ paddingBottom: Math.max(insetBottom, 8) }}>
      <View
        className="flex-row items-center rounded-full px-3 min-h-[36px] max-h-[100px]"
        style={{
          ...glassCard,
          backgroundColor: glassCard.background,
          borderColor: 'rgba(139, 92, 246, 0.4)',
          borderRadius: 9999,
        }}
      >
        {/* Paperclip icon per Stitch spec */}
        <TouchableOpacity className="w-8 h-8 items-center justify-center mr-1">
          <Ionicons name="attach-outline" size={22} color={colors.text.tertiary} />
        </TouchableOpacity>
        <TextInput
          ref={inputRef}
          className="flex-1 text-base text-text-primary py-2 max-h-[100px]"
          placeholder={isListening ? 'Listening...' : 'Message Pierre...'}
          placeholderTextColor={isListening ? colors.error : colors.text.tertiary}
          value={displayText}
          onChangeText={onChangeText}
          multiline
          maxLength={4000}
          returnKeyType="default"
          editable={!isListening}
          testID="message-input"
        />
        <VoiceButton
          isListening={isListening}
          isAvailable={voiceAvailable}
          onPress={onVoicePress}
          disabled={isSending}
          size="sm"
          testID="voice-input-button"
        />
        {/* Violet send button per Stitch spec */}
        <TouchableOpacity
          className={`w-9 h-9 rounded-full items-center justify-center ml-2 ${
            !canSend ? 'bg-background-tertiary' : ''
          }`}
          style={canSend ? { backgroundColor: colors.pierre.violet } : undefined}
          onPress={onSendMessage}
          disabled={!canSend}
          testID="send-button"
        >
          {isSending ? (
            <ActivityIndicator size="small" color={colors.text.primary} />
          ) : (
            <Ionicons name="arrow-up" size={20} color={colors.text.primary} />
          )}
        </TouchableOpacity>
      </View>
      {isListening && (
        <View className="pt-1 items-center">
          <Text className="text-xs text-error">Tap mic to stop recording</Text>
        </View>
      )}
    </View>
  );
}
