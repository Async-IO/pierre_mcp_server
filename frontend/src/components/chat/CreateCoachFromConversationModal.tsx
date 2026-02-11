// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Modal for generating a coach from conversation history using LLM analysis
// ABOUTME: Shows analysis state, pre-fills form with suggestions, allows editing and saving

import { useState, useEffect, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { coachesApi } from '../../services/api';
import type { CoachFormData } from './types';
import { Sparkles, RefreshCw, AlertCircle, MessageSquareText } from 'lucide-react';
import { QUERY_KEYS } from '../../constants/queryKeys';

interface CreateCoachFromConversationModalProps {
  isOpen: boolean;
  conversationId: string;
  messageCount: number;
  onClose: () => void;
  onSuccess: () => void;
}

type AnalysisState = 'idle' | 'analyzing' | 'ready' | 'error';

const MAX_MESSAGES_ANALYZED = 10;

export default function CreateCoachFromConversationModal({
  isOpen,
  conversationId,
  messageCount,
  onClose,
  onSuccess,
}: CreateCoachFromConversationModalProps) {
  const queryClient = useQueryClient();
  const [analysisState, setAnalysisState] = useState<AnalysisState>('idle');
  const [messagesAnalyzed, setMessagesAnalyzed] = useState(0);
  const [totalMessages, setTotalMessages] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [formData, setFormData] = useState<CoachFormData>({
    title: '',
    description: '',
    system_prompt: '',
    category: 'Custom',
  });

  const generateMutation = useMutation({
    mutationFn: () =>
      coachesApi.generateFromConversation({
        conversation_id: conversationId,
        max_messages: MAX_MESSAGES_ANALYZED,
      }),
    onSuccess: (data) => {
      setFormData({
        title: data.title,
        description: data.description,
        system_prompt: data.system_prompt,
        category: data.category,
      });
      setMessagesAnalyzed(data.messages_analyzed);
      setTotalMessages(data.total_messages);
      setAnalysisState('ready');
      setErrorMessage(null);
    },
    onError: (error) => {
      const message = error instanceof Error ? error.message : 'Failed to analyze conversation';
      setErrorMessage(message);
      setAnalysisState('error');
    },
  });

  const createMutation = useMutation({
    mutationFn: (data: CoachFormData) => coachesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      onSuccess();
    },
  });

  const startAnalysis = useCallback(() => {
    setAnalysisState('analyzing');
    setErrorMessage(null);
    generateMutation.mutate();
  }, [generateMutation]);

  // Start analysis when modal opens
  useEffect(() => {
    if (isOpen && analysisState === 'idle') {
      startAnalysis();
    }
  }, [isOpen, analysisState, startAnalysis]);

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setAnalysisState('idle');
      setErrorMessage(null);
      setFormData({
        title: '',
        description: '',
        system_prompt: '',
        category: 'Custom',
      });
      setMessagesAnalyzed(0);
      setTotalMessages(0);
    }
  }, [isOpen]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.title.trim() || !formData.system_prompt.trim()) return;
    createMutation.mutate(formData);
  };

  const handleRegenerate = () => {
    startAnalysis();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />
      {/* Modal Content */}
      <div className="relative bg-white rounded-2xl shadow-2xl max-w-lg w-full mx-4 max-h-[90vh] overflow-y-auto">
        <div className="p-6">
          {/* Close button */}
          <button
            onClick={onClose}
            className="absolute top-4 right-4 p-2 text-pierre-gray-400 hover:text-pierre-gray-600 hover:bg-pierre-gray-100 rounded-lg transition-colors"
            aria-label="Close"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>

          <div className="text-center mb-6">
            <div className="w-12 h-12 bg-gradient-to-br from-pierre-violet to-pierre-cyan rounded-xl flex items-center justify-center mx-auto mb-4">
              <Sparkles className="w-6 h-6 text-white" />
            </div>
            <h2 className="text-xl font-semibold text-pierre-gray-900 mb-2">
              Create Coach from Conversation
            </h2>
            <p className="text-pierre-gray-500 text-sm">
              AI analyzes your conversation to generate a specialized coach
            </p>
          </div>

          {/* Analyzing State */}
          {analysisState === 'analyzing' && (
            <div className="text-center py-8">
              <div className="w-16 h-16 mx-auto mb-4 relative">
                <div className="absolute inset-0 bg-pierre-violet/10 rounded-full animate-ping" />
                <div className="relative w-16 h-16 bg-pierre-violet/10 rounded-full flex items-center justify-center">
                  <MessageSquareText className="w-8 h-8 text-pierre-violet animate-pulse" />
                </div>
              </div>
              <p className="text-pierre-gray-700 font-medium mb-2">Analyzing conversation...</p>
              <p className="text-pierre-gray-500 text-sm">
                Reading last {Math.min(messageCount, MAX_MESSAGES_ANALYZED)} of {messageCount} messages
              </p>
            </div>
          )}

          {/* Error State */}
          {analysisState === 'error' && (
            <div className="text-center py-8">
              <div className="w-16 h-16 mx-auto mb-4 bg-red-50 rounded-full flex items-center justify-center">
                <AlertCircle className="w-8 h-8 text-red-500" />
              </div>
              <p className="text-pierre-gray-700 font-medium mb-2">Analysis Failed</p>
              <p className="text-pierre-gray-500 text-sm mb-4">{errorMessage}</p>
              <button
                onClick={handleRegenerate}
                className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet/90 transition-colors"
              >
                <RefreshCw className="w-4 h-4" />
                Try Again
              </button>
            </div>
          )}

          {/* Ready State - Form */}
          {analysisState === 'ready' && (
            <>
              {/* Analysis Info Banner */}
              <div className="mb-4 p-3 bg-pierre-violet/5 border border-pierre-violet/10 rounded-lg">
                <div className="flex items-center gap-2 text-sm text-pierre-violet">
                  <MessageSquareText className="w-4 h-4" />
                  <span>
                    Analyzed {messagesAnalyzed} of {totalMessages} messages
                  </span>
                  <button
                    onClick={handleRegenerate}
                    disabled={generateMutation.isPending}
                    className="ml-auto p-1.5 hover:bg-pierre-violet/10 rounded-lg transition-colors disabled:opacity-50"
                    title="Regenerate suggestions"
                  >
                    <RefreshCw className={`w-4 h-4 ${generateMutation.isPending ? 'animate-spin' : ''}`} />
                  </button>
                </div>
              </div>

              <form onSubmit={handleSubmit} className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Coach Name
                  </label>
                  <input
                    type="text"
                    placeholder="e.g., Marathon Training Coach"
                    value={formData.title}
                    onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                    required
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Description <span className="text-pierre-gray-400">(optional)</span>
                  </label>
                  <input
                    type="text"
                    placeholder="Brief description of what this coach specializes in"
                    value={formData.description}
                    onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    System Prompt
                  </label>
                  <textarea
                    placeholder="Define your coach's personality, expertise, and communication style..."
                    value={formData.system_prompt}
                    onChange={(e) => setFormData({ ...formData, system_prompt: e.target.value })}
                    rows={6}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent resize-none"
                    required
                  />
                  {formData.system_prompt && (
                    <p className="text-xs text-pierre-gray-500 mt-1">
                      ~{Math.ceil(formData.system_prompt.length / 4)} tokens ({((Math.ceil(formData.system_prompt.length / 4) / 128000) * 100).toFixed(1)}% of context)
                    </p>
                  )}
                </div>

                <div>
                  <label className="block text-sm font-medium text-pierre-gray-700 mb-1">
                    Category
                  </label>
                  <select
                    value={formData.category}
                    onChange={(e) => setFormData({ ...formData, category: e.target.value })}
                    className="w-full px-3 py-2 text-sm border border-pierre-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:border-transparent bg-white"
                  >
                    <option value="Training">Training</option>
                    <option value="Nutrition">Nutrition</option>
                    <option value="Recovery">Recovery</option>
                    <option value="Recipes">Recipes</option>
                    <option value="Mobility">Mobility</option>
                    <option value="Analysis">Analysis</option>
                    <option value="Custom">Custom</option>
                  </select>
                </div>

                <div className="flex gap-3 pt-2">
                  <button
                    type="button"
                    onClick={onClose}
                    className="flex-1 px-4 py-2 text-sm font-medium text-pierre-gray-600 bg-pierre-gray-100 rounded-lg hover:bg-pierre-gray-200 transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={createMutation.isPending || !formData.title.trim() || !formData.system_prompt.trim()}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-pierre-violet rounded-lg hover:bg-pierre-violet/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                  >
                    {createMutation.isPending ? 'Saving...' : 'Save Coach'}
                  </button>
                </div>

                {createMutation.isError && (
                  <p className="text-xs text-pierre-red-500 text-center">
                    Failed to create coach. Please try again.
                  </p>
                )}
              </form>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
