// ABOUTME: Centralized exports for custom React hooks
// ABOUTME: Provides voice input, activities, and other reusable hook functionality

export { useVoiceInput } from './useVoiceInput';
export type { VoiceError, VoiceErrorType } from './useVoiceInput';

// Activity and training data hooks with offline caching
export {
  useActivities,
  useTrainingLoad,
  useRecoveryScore,
} from './useActivities';
export type {
  Activity,
  TrainingLoadData,
  RecoveryScore,
  ActivitiesParams,
} from './useActivities';
