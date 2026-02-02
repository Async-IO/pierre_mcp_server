// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: 7-step wizard for creating/editing AI coaches (ASY-148)
// ABOUTME: Includes markdown toggle, live preview, token counter, and export/import

import React, { useState, useCallback, useMemo } from 'react';
import { CoachVersionHistory } from './CoachVersionHistory';

// Types
interface CoachWizardProps {
  coachId?: string;
  onSave: (coach: CoachFormData) => Promise<void>;
  onCancel: () => void;
  initialData?: CoachFormData;
}

interface CoachFormData {
  title: string;
  description: string;
  category: CoachCategory;
  tags: string[];
  purpose: string;
  whenToUse: string;
  systemPrompt: string;
  exampleInputs: string;
  exampleOutputs: string;
  prerequisites: CoachPrerequisites;
  successCriteria: string;
  relatedCoaches: string[];
}

interface CoachPrerequisites {
  providers: string[];
  minActivities: number;
  activityTypes: string[];
}

type CoachCategory = 'training' | 'nutrition' | 'recovery' | 'recipes' | 'mobility' | 'custom';

const STEPS = [
  { id: 'basic', title: 'Basic Info', description: 'Name and categorize your coach' },
  { id: 'purpose', title: 'Purpose', description: 'Define what and when' },
  { id: 'prompt', title: 'System Prompt', description: 'Core instructions' },
  { id: 'examples', title: 'Examples', description: 'Input/output samples' },
  { id: 'prerequisites', title: 'Prerequisites', description: 'Required connections' },
  { id: 'advanced', title: 'Advanced', description: 'Related coaches & criteria' },
  { id: 'review', title: 'Review', description: 'Final check' },
];

const CATEGORIES: Array<{ key: CoachCategory; label: string; color: string }> = [
  { key: 'training', label: 'Training', color: '#4ADE80' },
  { key: 'nutrition', label: 'Nutrition', color: '#F59E0B' },
  { key: 'recovery', label: 'Recovery', color: '#6366F1' },
  { key: 'recipes', label: 'Recipes', color: '#F97316' },
  { key: 'mobility', label: 'Mobility', color: '#EC4899' },
  { key: 'custom', label: 'Custom', color: '#8B5CF6' },
];

const PROVIDERS = ['Strava', 'Garmin', 'Fitbit', 'Whoop', 'Coros', 'Terra'];
const ACTIVITY_TYPES = ['Run', 'Ride', 'Swim', 'Walk', 'Hike', 'Workout', 'Yoga'];

const CONTEXT_WINDOW_SIZE = 128000;

const defaultFormData: CoachFormData = {
  title: '',
  description: '',
  category: 'custom',
  tags: [],
  purpose: '',
  whenToUse: '',
  systemPrompt: '',
  exampleInputs: '',
  exampleOutputs: '',
  prerequisites: {
    providers: [],
    minActivities: 0,
    activityTypes: [],
  },
  successCriteria: '',
  relatedCoaches: [],
};

export function CoachWizard({ coachId, onSave, onCancel, initialData }: CoachWizardProps) {
  const isEditMode = Boolean(coachId);
  const [currentStep, setCurrentStep] = useState(0);
  const [formData, setFormData] = useState<CoachFormData>(initialData || defaultFormData);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isMarkdownMode, setIsMarkdownMode] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [newTag, setNewTag] = useState('');
  const [showVersionHistory, setShowVersionHistory] = useState(false);

  // Calculate token counts
  const tokenCounts = useMemo(() => {
    const systemPromptTokens = Math.ceil(formData.systemPrompt.length / 4);
    const purposeTokens = Math.ceil(formData.purpose.length / 4);
    const examplesTokens = Math.ceil((formData.exampleInputs.length + formData.exampleOutputs.length) / 4);
    const total = systemPromptTokens + purposeTokens + examplesTokens;
    return {
      systemPrompt: systemPromptTokens,
      purpose: purposeTokens,
      examples: examplesTokens,
      total,
      percentage: ((total / CONTEXT_WINDOW_SIZE) * 100).toFixed(1),
    };
  }, [formData]);

  // Update form field
  const updateField = useCallback(<K extends keyof CoachFormData>(field: K, value: CoachFormData[K]) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    setErrors(prev => ({ ...prev, [field]: '' }));
  }, []);

  // Validate current step
  const validateStep = useCallback((step: number): boolean => {
    const newErrors: Record<string, string> = {};

    switch (step) {
      case 0: // Basic Info
        if (!formData.title.trim()) newErrors.title = 'Title is required';
        if (formData.title.length > 100) newErrors.title = 'Title must be 100 characters or less';
        break;
      case 2: // System Prompt
        if (!formData.systemPrompt.trim()) newErrors.systemPrompt = 'System prompt is required';
        break;
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [formData]);

  // Navigation
  const goToStep = (step: number) => {
    if (step < 0 || step >= STEPS.length) return;
    if (step > currentStep && !validateStep(currentStep)) return;
    setCurrentStep(step);
  };

  const handleNext = () => goToStep(currentStep + 1);
  const handleBack = () => goToStep(currentStep - 1);

  // Tag management
  const addTag = () => {
    const trimmed = newTag.trim().toLowerCase();
    if (trimmed && !formData.tags.includes(trimmed) && formData.tags.length < 10) {
      updateField('tags', [...formData.tags, trimmed]);
      setNewTag('');
    }
  };

  const removeTag = (tag: string) => {
    updateField('tags', formData.tags.filter(t => t !== tag));
  };

  // Prerequisites management
  const toggleProvider = (provider: string) => {
    const current = formData.prerequisites.providers;
    const updated = current.includes(provider)
      ? current.filter(p => p !== provider)
      : [...current, provider];
    updateField('prerequisites', { ...formData.prerequisites, providers: updated });
  };

  const toggleActivityType = (type: string) => {
    const current = formData.prerequisites.activityTypes;
    const updated = current.includes(type)
      ? current.filter(t => t !== type)
      : [...current, type];
    updateField('prerequisites', { ...formData.prerequisites, activityTypes: updated });
  };

  // Export to markdown
  const exportToMarkdown = () => {
    const markdown = `---
title: ${formData.title}
category: ${formData.category}
tags: [${formData.tags.join(', ')}]
prerequisites:
  providers: [${formData.prerequisites.providers.join(', ')}]
  min_activities: ${formData.prerequisites.minActivities}
  activity_types: [${formData.prerequisites.activityTypes.join(', ')}]
---

## Purpose

${formData.purpose}

## When to Use

${formData.whenToUse}

## Instructions

${formData.systemPrompt}

## Example Inputs

${formData.exampleInputs}

## Example Outputs

${formData.exampleOutputs}

## Success Criteria

${formData.successCriteria}
`;

    const blob = new Blob([markdown], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${formData.title.toLowerCase().replace(/\s+/g, '-')}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Import from markdown
  const importFromMarkdown = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      // Simple YAML frontmatter parser
      const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
      if (frontmatterMatch) {
        const frontmatter = frontmatterMatch[1];
        // Extract title
        const titleMatch = frontmatter.match(/title:\s*(.+)/);
        if (titleMatch) updateField('title', titleMatch[1].trim());
        // Extract category
        const categoryMatch = frontmatter.match(/category:\s*(.+)/);
        if (categoryMatch) updateField('category', categoryMatch[1].trim() as CoachCategory);
        // Extract tags
        const tagsMatch = frontmatter.match(/tags:\s*\[([^\]]*)\]/);
        if (tagsMatch) {
          const tags = tagsMatch[1].split(',').map(t => t.trim()).filter(Boolean);
          updateField('tags', tags);
        }
      }

      // Extract sections
      const purposeMatch = content.match(/## Purpose\n\n([\s\S]*?)(?=\n## |$)/);
      if (purposeMatch) updateField('purpose', purposeMatch[1].trim());

      const whenMatch = content.match(/## When to Use\n\n([\s\S]*?)(?=\n## |$)/);
      if (whenMatch) updateField('whenToUse', whenMatch[1].trim());

      const instructionsMatch = content.match(/## Instructions\n\n([\s\S]*?)(?=\n## |$)/);
      if (instructionsMatch) updateField('systemPrompt', instructionsMatch[1].trim());

      const inputsMatch = content.match(/## Example Inputs\n\n([\s\S]*?)(?=\n## |$)/);
      if (inputsMatch) updateField('exampleInputs', inputsMatch[1].trim());

      const outputsMatch = content.match(/## Example Outputs\n\n([\s\S]*?)(?=\n## |$)/);
      if (outputsMatch) updateField('exampleOutputs', outputsMatch[1].trim());

      const criteriaMatch = content.match(/## Success Criteria\n\n([\s\S]*?)(?=\n## |$)/);
      if (criteriaMatch) updateField('successCriteria', criteriaMatch[1].trim());
    };
    reader.readAsText(file);
  };

  // Save handler
  const handleSave = async () => {
    if (!validateStep(currentStep)) return;
    setIsSaving(true);
    try {
      await onSave(formData);
    } catch (error) {
      console.error('Failed to save coach:', error);
    } finally {
      setIsSaving(false);
    }
  };

  // Render step indicator
  const renderStepIndicator = () => (
    <div className="flex items-center justify-between mb-8 px-4">
      {STEPS.map((step, index) => (
        <React.Fragment key={step.id}>
          <button
            onClick={() => goToStep(index)}
            className={`flex flex-col items-center group ${
              index <= currentStep ? 'cursor-pointer' : 'cursor-not-allowed opacity-50'
            }`}
            disabled={index > currentStep + 1}
          >
            <div
              className={`w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold transition-colors ${
                index === currentStep
                  ? 'bg-pierre-violet text-white'
                  : index < currentStep
                  ? 'bg-pierre-activity text-white'
                  : 'bg-pierre-gray-200 text-pierre-gray-500'
              }`}
            >
              {index < currentStep ? '✓' : index + 1}
            </div>
            <span className={`text-xs mt-1 ${index === currentStep ? 'text-pierre-violet font-medium' : 'text-pierre-gray-500'}`}>
              {step.title}
            </span>
          </button>
          {index < STEPS.length - 1 && (
            <div className={`flex-1 h-0.5 mx-2 ${index < currentStep ? 'bg-pierre-activity' : 'bg-pierre-gray-200'}`} />
          )}
        </React.Fragment>
      ))}
    </div>
  );

  // Step 1: Basic Info
  const renderBasicInfo = () => (
    <div className="space-y-6">
      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Title *</label>
        <input
          type="text"
          value={formData.title}
          onChange={(e) => updateField('title', e.target.value)}
          placeholder="Enter coach title"
          className={`w-full px-4 py-3 bg-white border rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 ${
            errors.title ? 'border-pierre-red-500' : 'border-pierre-gray-300'
          }`}
          maxLength={100}
        />
        <div className="flex justify-between mt-1">
          {errors.title && <span className="text-pierre-red-500 text-xs">{errors.title}</span>}
          <span className="text-pierre-gray-500 text-xs ml-auto">{formData.title.length}/100</span>
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Description</label>
        <textarea
          value={formData.description}
          onChange={(e) => updateField('description', e.target.value)}
          placeholder="Brief description of what this coach does"
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={3}
          maxLength={500}
        />
        <span className="text-pierre-gray-500 text-xs float-right">{formData.description.length}/500</span>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Category</label>
        <div className="grid grid-cols-3 gap-3">
          {CATEGORIES.map((cat) => (
            <button
              key={cat.key}
              onClick={() => updateField('category', cat.key)}
              className={`px-4 py-3 rounded-lg border-2 transition-colors ${
                formData.category === cat.key
                  ? 'border-pierre-violet bg-pierre-violet/10'
                  : 'border-pierre-gray-200 hover:border-pierre-gray-300'
              }`}
            >
              <span className="inline-block w-3 h-3 rounded-full mr-2" style={{ backgroundColor: cat.color }} />
              <span className="text-pierre-gray-800">{cat.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Tags</label>
        <div className="flex gap-2 mb-2">
          <input
            type="text"
            value={newTag}
            onChange={(e) => setNewTag(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && (e.preventDefault(), addTag())}
            placeholder="Add a tag"
            className="flex-1 px-4 py-2 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50"
          />
          <button
            onClick={addTag}
            className="px-4 py-2 bg-pierre-violet text-white rounded-lg hover:bg-pierre-violet-dark transition-colors"
          >
            Add
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          {formData.tags.map((tag) => (
            <span key={tag} className="inline-flex items-center px-3 py-1 rounded-full bg-pierre-gray-100 text-pierre-gray-700">
              {tag}
              <button onClick={() => removeTag(tag)} className="ml-2 text-pierre-gray-400 hover:text-pierre-gray-700">×</button>
            </span>
          ))}
          {formData.tags.length === 0 && <span className="text-pierre-gray-500 text-sm italic">No tags added</span>}
        </div>
      </div>
    </div>
  );

  // Step 2: Purpose
  const renderPurpose = () => (
    <div className="space-y-6">
      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Purpose</label>
        <textarea
          value={formData.purpose}
          onChange={(e) => updateField('purpose', e.target.value)}
          placeholder="Describe what this coach helps users accomplish..."
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={5}
        />
        <span className="text-pierre-gray-500 text-xs float-right">~{tokenCounts.purpose} tokens</span>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">When to Use</label>
        <textarea
          value={formData.whenToUse}
          onChange={(e) => updateField('whenToUse', e.target.value)}
          placeholder="Describe scenarios when users should activate this coach..."
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={4}
        />
      </div>
    </div>
  );

  // Step 3: System Prompt
  const renderSystemPrompt = () => (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <label className="block text-sm font-medium text-pierre-gray-700">System Prompt *</label>
        <div className="flex gap-2">
          <button
            onClick={() => setIsMarkdownMode(!isMarkdownMode)}
            className={`px-3 py-1 text-sm rounded ${
              isMarkdownMode ? 'bg-pierre-violet text-white' : 'bg-pierre-gray-100 text-pierre-gray-700'
            }`}
          >
            {isMarkdownMode ? 'Visual' : 'Markdown'}
          </button>
          <button
            onClick={() => setShowPreview(!showPreview)}
            className={`px-3 py-1 text-sm rounded ${
              showPreview ? 'bg-pierre-violet text-white' : 'bg-pierre-gray-100 text-pierre-gray-700'
            }`}
          >
            Preview
          </button>
        </div>
      </div>

      <div className={`grid ${showPreview ? 'grid-cols-2 gap-4' : 'grid-cols-1'}`}>
        <div>
          <textarea
            value={formData.systemPrompt}
            onChange={(e) => updateField('systemPrompt', e.target.value)}
            placeholder="Define how this coach should behave, what expertise it has, and how it should respond to users..."
            className={`w-full px-4 py-3 bg-white border rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none font-mono ${
              errors.systemPrompt ? 'border-pierre-red-500' : 'border-pierre-gray-300'
            }`}
            rows={15}
          />
          {errors.systemPrompt && <span className="text-pierre-red-500 text-xs">{errors.systemPrompt}</span>}
        </div>
        {showPreview && (
          <div className="px-4 py-3 bg-pierre-gray-50 border border-pierre-gray-200 rounded-lg overflow-auto max-h-[400px]">
            <div className="prose prose-sm">
              <pre className="whitespace-pre-wrap text-pierre-gray-700 text-sm">{formData.systemPrompt}</pre>
            </div>
          </div>
        )}
      </div>

      <div className="flex items-center justify-between">
        <div className="text-sm text-pierre-gray-500">
          ~{tokenCounts.systemPrompt.toLocaleString()} tokens ({tokenCounts.percentage}% of context)
        </div>
        <div className="w-48 h-2 bg-pierre-gray-200 rounded-full overflow-hidden">
          <div
            className="h-full bg-pierre-violet transition-all"
            style={{ width: `${Math.min(parseFloat(tokenCounts.percentage), 100)}%` }}
          />
        </div>
      </div>
    </div>
  );

  // Step 4: Examples
  const renderExamples = () => (
    <div className="space-y-6">
      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Example Inputs</label>
        <textarea
          value={formData.exampleInputs}
          onChange={(e) => updateField('exampleInputs', e.target.value)}
          placeholder="List example prompts users might ask this coach..."
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={6}
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Example Outputs</label>
        <textarea
          value={formData.exampleOutputs}
          onChange={(e) => updateField('exampleOutputs', e.target.value)}
          placeholder="Show example responses the coach should provide..."
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={6}
        />
      </div>

      <div className="text-sm text-pierre-gray-500">
        Examples contribute ~{tokenCounts.examples.toLocaleString()} tokens
      </div>
    </div>
  );

  // Step 5: Prerequisites
  const renderPrerequisites = () => (
    <div className="space-y-6">
      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-3">Required Providers</label>
        <p className="text-pierre-gray-500 text-sm mb-3">Users must have these connected to use this coach</p>
        <div className="flex flex-wrap gap-2">
          {PROVIDERS.map((provider) => (
            <button
              key={provider}
              onClick={() => toggleProvider(provider)}
              className={`px-4 py-2 rounded-lg border transition-colors ${
                formData.prerequisites.providers.includes(provider)
                  ? 'border-pierre-violet bg-pierre-violet/10 text-pierre-violet-light'
                  : 'border-pierre-gray-200 text-pierre-gray-600 hover:border-pierre-gray-300'
              }`}
            >
              {provider}
            </button>
          ))}
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Minimum Activities</label>
        <input
          type="number"
          min="0"
          value={formData.prerequisites.minActivities}
          onChange={(e) => updateField('prerequisites', {
            ...formData.prerequisites,
            minActivities: parseInt(e.target.value) || 0,
          })}
          className="w-32 px-4 py-2 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50"
        />
        <span className="text-pierre-gray-500 text-sm ml-2">activities required</span>
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-3">Activity Types</label>
        <div className="flex flex-wrap gap-2">
          {ACTIVITY_TYPES.map((type) => (
            <button
              key={type}
              onClick={() => toggleActivityType(type)}
              className={`px-4 py-2 rounded-lg border transition-colors ${
                formData.prerequisites.activityTypes.includes(type)
                  ? 'border-pierre-activity bg-pierre-activity/10 text-pierre-activity'
                  : 'border-pierre-gray-200 text-pierre-gray-600 hover:border-pierre-gray-300'
              }`}
            >
              {type}
            </button>
          ))}
        </div>
      </div>
    </div>
  );

  // Step 6: Advanced
  const renderAdvanced = () => (
    <div className="space-y-6">
      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Success Criteria</label>
        <textarea
          value={formData.successCriteria}
          onChange={(e) => updateField('successCriteria', e.target.value)}
          placeholder="Define what successful coaching looks like..."
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50 resize-none"
          rows={4}
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-pierre-gray-700 mb-2">Related Coaches</label>
        <p className="text-pierre-gray-500 text-sm mb-3">Suggest other coaches users might find helpful</p>
        <input
          type="text"
          placeholder="Enter coach names separated by commas"
          value={formData.relatedCoaches.join(', ')}
          onChange={(e) => updateField('relatedCoaches', e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
          className="w-full px-4 py-3 bg-white border border-pierre-gray-300 rounded-lg text-pierre-gray-900 placeholder-pierre-gray-400 focus:outline-none focus:ring-2 focus:ring-pierre-violet focus:ring-opacity-50"
        />
      </div>
    </div>
  );

  // Step 7: Review
  const renderReview = () => {
    const selectedCategory = CATEGORIES.find(c => c.key === formData.category);

    return (
      <div className="space-y-6">
        <h3 className="text-xl font-bold text-pierre-gray-900">Review Your Coach</h3>

        <div className="grid grid-cols-2 gap-6">
          <div className="space-y-4">
            <div>
              <span className="text-pierre-gray-500 text-sm">Title</span>
              <p className="text-pierre-gray-900 font-medium">{formData.title || '(not set)'}</p>
            </div>

            <div>
              <span className="text-pierre-gray-500 text-sm">Category</span>
              <p>
                <span
                  className="inline-block px-3 py-1 rounded-full text-white text-sm"
                  style={{ backgroundColor: selectedCategory?.color }}
                >
                  {selectedCategory?.label}
                </span>
              </p>
            </div>

            {formData.tags.length > 0 && (
              <div>
                <span className="text-pierre-gray-500 text-sm">Tags</span>
                <div className="flex flex-wrap gap-2 mt-1">
                  {formData.tags.map((tag) => (
                    <span key={tag} className="px-2 py-1 bg-pierre-gray-100 rounded text-pierre-gray-700 text-sm">
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {formData.prerequisites.providers.length > 0 && (
              <div>
                <span className="text-pierre-gray-500 text-sm">Required Providers</span>
                <p className="text-pierre-gray-900">{formData.prerequisites.providers.join(', ')}</p>
              </div>
            )}
          </div>

          <div className="space-y-4">
            <div>
              <span className="text-pierre-gray-500 text-sm">Token Usage</span>
              <p className="text-pierre-gray-900">~{tokenCounts.total.toLocaleString()} tokens ({tokenCounts.percentage}%)</p>
            </div>

            {formData.description && (
              <div>
                <span className="text-pierre-gray-500 text-sm">Description</span>
                <p className="text-pierre-gray-700 text-sm">{formData.description}</p>
              </div>
            )}

            {formData.purpose && (
              <div>
                <span className="text-pierre-gray-500 text-sm">Purpose</span>
                <p className="text-pierre-gray-700 text-sm line-clamp-3">{formData.purpose}</p>
              </div>
            )}
          </div>
        </div>

        <div className="p-4 bg-pierre-gray-50 rounded-lg border border-pierre-gray-200">
          <span className="text-pierre-gray-500 text-sm">System Prompt Preview</span>
          <pre className="text-pierre-gray-700 text-sm mt-2 whitespace-pre-wrap line-clamp-6 font-mono">
            {formData.systemPrompt || '(not set)'}
          </pre>
        </div>
      </div>
    );
  };

  // Render current step content
  const renderStepContent = () => {
    switch (currentStep) {
      case 0: return renderBasicInfo();
      case 1: return renderPurpose();
      case 2: return renderSystemPrompt();
      case 3: return renderExamples();
      case 4: return renderPrerequisites();
      case 5: return renderAdvanced();
      case 6: return renderReview();
      default: return null;
    }
  };

  return (
    <div className="fixed inset-0 bg-white z-50 overflow-hidden flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-pierre-gray-200">
        <button onClick={onCancel} className="text-pierre-gray-500 hover:text-pierre-gray-900 transition-colors">
          Cancel
        </button>
        <h2 className="text-lg font-semibold text-pierre-gray-900">
          {isEditMode ? 'Edit Coach' : 'Create Coach'}
        </h2>
        <div className="flex gap-2">
          {isEditMode && (
            <button
              onClick={() => setShowVersionHistory(true)}
              className="px-3 py-1 text-sm bg-pierre-gray-100 text-pierre-gray-700 rounded hover:bg-pierre-gray-200 transition-colors"
            >
              History
            </button>
          )}
          <label className="cursor-pointer px-3 py-1 text-sm bg-pierre-gray-100 text-pierre-gray-700 rounded hover:bg-pierre-gray-200 transition-colors">
            Import
            <input type="file" accept=".md" onChange={importFromMarkdown} className="hidden" />
          </label>
          <button
            onClick={exportToMarkdown}
            className="px-3 py-1 text-sm bg-pierre-gray-100 text-pierre-gray-700 rounded hover:bg-pierre-gray-200 transition-colors"
          >
            Export
          </button>
        </div>
      </div>

      {/* Step Indicator */}
      <div className="px-6 py-6 border-b border-pierre-gray-200 bg-pierre-gray-50">
        {renderStepIndicator()}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-6 bg-pierre-gray-50">
        <div className="max-w-3xl mx-auto">
          <h3 className="text-lg font-semibold text-pierre-gray-900 mb-2">{STEPS[currentStep].title}</h3>
          <p className="text-pierre-gray-500 mb-6">{STEPS[currentStep].description}</p>
          {renderStepContent()}
        </div>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between px-6 py-4 border-t border-pierre-gray-200 bg-white">
        <button
          onClick={handleBack}
          disabled={currentStep === 0}
          className={`px-4 py-2 rounded-lg transition-colors ${
            currentStep === 0
              ? 'text-pierre-gray-300 cursor-not-allowed'
              : 'text-pierre-gray-500 hover:text-pierre-gray-900'
          }`}
        >
          ← Back
        </button>

        <div className="text-pierre-gray-500 text-sm">
          Step {currentStep + 1} of {STEPS.length}
        </div>

        {currentStep < STEPS.length - 1 ? (
          <button
            onClick={handleNext}
            className="px-6 py-2 bg-pierre-violet text-white rounded-lg hover:bg-pierre-violet-dark transition-colors"
          >
            Next →
          </button>
        ) : (
          <button
            onClick={handleSave}
            disabled={isSaving}
            className="px-6 py-2 bg-pierre-activity text-white rounded-lg hover:bg-pierre-activity-dark transition-colors disabled:opacity-50"
          >
            {isSaving ? 'Saving...' : isEditMode ? 'Save Changes' : 'Create Coach'}
          </button>
        )}
      </div>

      {/* Version History Modal */}
      {isEditMode && coachId && (
        <CoachVersionHistory
          coachId={coachId}
          coachTitle={formData.title || 'Coach'}
          isOpen={showVersionHistory}
          onClose={() => setShowVersionHistory(false)}
          onReverted={() => {
            setShowVersionHistory(false);
            // Optionally refresh the form data after revert
          }}
        />
      )}
    </div>
  );
}

export default CoachWizard;
