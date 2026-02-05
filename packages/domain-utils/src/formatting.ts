// ABOUTME: Date and text formatting utilities shared across web and mobile
// ABOUTME: Pure functions with no platform-specific dependencies

/**
 * Format a date string for conversation list display
 * Shows time for today, "Yesterday", weekday for last week, or date
 */
export function formatRelativeDate(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  } else if (diffDays === 1) {
    return 'Yesterday';
  } else if (diffDays < 7) {
    return date.toLocaleDateString('en-US', { weekday: 'short' });
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }
}

/**
 * Format a date for full display (e.g., "January 15, 2025")
 */
export function formatFullDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

/**
 * Format duration in seconds to human readable string
 * e.g., 3665 -> "1h 1m 5s"
 */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs}s`);

  return parts.join(' ');
}

/**
 * Format distance in meters to human readable string
 * Uses km for >= 1000m, otherwise meters
 */
export function formatDistance(meters: number, unit: 'metric' | 'imperial' = 'metric'): string {
  if (unit === 'imperial') {
    const miles = meters / 1609.344;
    if (miles >= 1) {
      return `${miles.toFixed(2)} mi`;
    }
    const feet = meters * 3.28084;
    return `${Math.round(feet)} ft`;
  }

  if (meters >= 1000) {
    return `${(meters / 1000).toFixed(2)} km`;
  }
  return `${Math.round(meters)} m`;
}

/**
 * Format pace (seconds per km or mile) to human readable string
 * e.g., 300 -> "5:00 /km"
 */
export function formatPace(secondsPerUnit: number, unit: 'metric' | 'imperial' = 'metric'): string {
  const minutes = Math.floor(secondsPerUnit / 60);
  const seconds = Math.floor(secondsPerUnit % 60);
  const unitLabel = unit === 'imperial' ? '/mi' : '/km';
  return `${minutes}:${seconds.toString().padStart(2, '0')} ${unitLabel}`;
}

/**
 * Truncate text to a maximum length with ellipsis
 */
export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 3) + '...';
}
