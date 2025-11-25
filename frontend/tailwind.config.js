/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Pierre brand colors - Holistic Intelligence design system
        pierre: {
          // Primary brand colors
          violet: '#7C3AED',  // Intelligence, AI, sophistication
          cyan: '#06B6D4',    // Data flow, connectivity, freshness

          // Three Pillars - Semantic accent colors
          activity: '#10B981',   // Emerald - Movement, fitness, energy
          nutrition: '#F59E0B',  // Amber - Food, fuel, nourishment
          recovery: '#6366F1',   // Indigo - Rest, sleep, restoration

          // Dark theme backgrounds
          dark: '#0F0F1A',       // Deep Space - Primary dark bg
          slate: '#1E1E2E',      // Secondary dark bg

          // Extended violet palette
          'violet-light': '#A78BFA',
          'violet-dark': '#5B21B6',

          // Extended cyan palette
          'cyan-light': '#22D3EE',
          'cyan-dark': '#0891B2',

          // Extended activity (emerald) palette
          'activity-light': '#34D399',
          'activity-dark': '#059669',

          // Extended nutrition (amber) palette
          'nutrition-light': '#FBBF24',
          'nutrition-dark': '#D97706',

          // Extended recovery (indigo) palette
          'recovery-light': '#818CF8',
          'recovery-dark': '#4F46E5',

          // Legacy color scales (for backward compatibility)
          blue: {
            50: '#eff6ff',
            100: '#dbeafe',
            200: '#bfdbfe',
            300: '#93c5fd',
            400: '#60a5fa',
            500: '#3b82f6',
            600: '#2563eb',
            700: '#1d4ed8',
            800: '#1e40af',
            900: '#1e3a8a',
          },
          gray: {
            50: '#f9fafb',
            100: '#f3f4f6',
            200: '#e5e7eb',
            300: '#d1d5db',
            400: '#9ca3af',
            500: '#6b7280',
            600: '#4b5563',
            700: '#374151',
            800: '#1f2937',
            900: '#111827',
          },
          green: {
            50: '#f0fdf4',
            100: '#dcfce7',
            500: '#22c55e',
            600: '#16a34a',
            700: '#15803d',
            800: '#166534',
          },
          yellow: {
            50: '#fefce8',
            100: '#fef3c7',
            500: '#eab308',
            600: '#ca8a04',
            700: '#a16207',
            800: '#854d0e',
          },
          red: {
            50: '#fef2f2',
            100: '#fee2e2',
            500: '#ef4444',
            600: '#dc2626',
            700: '#b91c1c',
            800: '#991b1b',
          },
          purple: {
            50: '#faf5ff',
            100: '#f3e8ff',
            500: '#a855f7',
            600: '#9333ea',
            700: '#7c3aed',
            800: '#6b21a8',
          },
          teal: {
            50: '#f0fdfa',
            100: '#ccfbf1',
            200: '#99f6e4',
            300: '#5eead4',
            400: '#2dd4bf',
            500: '#14b8a6',
            600: '#0d9488',
            700: '#0f766e',
            800: '#115e59',
          },
        },
        // API tier colors
        tier: {
          trial: '#eab308',
          starter: '#3b82f6',
          professional: '#22c55e',
          enterprise: '#a855f7',
        },
        // Legacy API colors for backward compatibility
        'api-blue': '#2563eb',
        'api-green': '#16a34a',
        'api-red': '#dc2626',
        'api-yellow': '#ca8a04',
      },
      fontFamily: {
        sans: ['system-ui', '-apple-system', 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', 'sans-serif'],
        mono: ['Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', 'monospace'],
      },
      fontSize: {
        'xs': '0.75rem',
        'sm': '0.875rem',
        'base': '1rem',
        'lg': '1.125rem',
        'xl': '1.25rem',
        '2xl': '1.5rem',
        '3xl': '1.875rem',
        '4xl': '2.25rem',
      },
      spacing: {
        '1': '0.25rem',
        '2': '0.5rem',
        '3': '0.75rem',
        '4': '1rem',
        '5': '1.25rem',
        '6': '1.5rem',
        '8': '2rem',
        '10': '2.5rem',
        '12': '3rem',
        '16': '4rem',
        '20': '5rem',
        '24': '6rem',
      },
      borderRadius: {
        'sm': '0.375rem',
        'md': '0.5rem',
        'lg': '0.75rem',
        'xl': '1rem',
      },
      boxShadow: {
        'sm': '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
        'md': '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
        'lg': '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
        'xl': '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
      },
      transitionDuration: {
        'fast': '150ms',
        'base': '200ms',
        'slow': '300ms',
      }
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}