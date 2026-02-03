// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Pierre brand colors - Holistic Intelligence design system
        pierre: {
          // Primary brand colors (synced with shared-constants/design-system.ts)
          violet: '#7C3AED',  // Intelligence, AI, sophistication (WCAG AA compliant with white text)
          cyan: '#22D3EE',    // Data flow, connectivity, freshness

          // Three Pillars - Semantic accent colors (synced with shared-constants)
          activity: '#4ADE80',   // Green - Movement, fitness, energy
          nutrition: '#F59E0B',  // Amber - Food, fuel, nourishment
          recovery: '#818CF8',   // Indigo - Rest, sleep, restoration
          mobility: '#EC4899',   // Pink - Flexibility, stretching, movement quality

          // Dark theme backgrounds
          dark: '#0F0F1A',       // Deep Space - Primary dark bg
          slate: '#1E1E2E',      // Secondary dark bg

          // Extended violet palette
          'violet-light': '#A78BFA',
          'violet-dark': '#7C3AED',

          // Extended cyan palette
          'cyan-light': '#67E8F9',
          'cyan-dark': '#06B6D4',

          // Extended activity (green) palette
          'activity-light': '#86EFAC',
          'activity-dark': '#22C55E',

          // Extended nutrition (amber) palette
          'nutrition-light': '#FCD34D',
          'nutrition-dark': '#D97706',

          // Extended recovery (indigo) palette
          'recovery-light': '#818CF8',
          'recovery-dark': '#4F46E5',

          // Extended mobility (pink) palette
          'mobility-light': '#F472B6',
          'mobility-dark': '#DB2777',

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
          pink: {
            50: '#fdf2f8',
            100: '#fce7f3',
            200: '#fbcfe8',
            500: '#ec4899',
            600: '#db2777',
            700: '#be185d',
            800: '#9d174d',
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
        // Plus Jakarta Sans for premium tech aesthetic, Inter as fallback
        sans: ['Plus Jakarta Sans', 'Inter', 'system-ui', '-apple-system', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', 'monospace'],
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
        // Glow effects for premium interactions (synced with shared-constants)
        'glow': '0 0 15px rgba(124, 58, 237, 0.3)',
        'glow-sm': '0 0 8px rgba(124, 58, 237, 0.2)',
        'glow-lg': '0 0 20px rgba(124, 58, 237, 0.5)',
        'glow-violet': '0 0 20px rgba(124, 58, 237, 0.3)',
        'glow-cyan': '0 0 20px rgba(34, 211, 238, 0.3)',
        'glow-activity': '0 0 20px rgba(74, 222, 128, 0.3)',
        'glow-nutrition': '0 0 20px rgba(245, 158, 11, 0.3)',
        'glow-recovery': '0 0 20px rgba(129, 140, 248, 0.3)',
        // Glassmorphism inner border effect
        'glass': 'inset 0 0 0 1px rgba(255, 255, 255, 0.1)',
      },
      backdropBlur: {
        xs: '2px',
      },
      transitionDuration: {
        'fast': '150ms',
        'base': '200ms',
        'slow': '300ms',
      },
      backgroundImage: {
        'gradient-pierre': 'linear-gradient(135deg, #8B5CF6 0%, #22D3EE 100%)',
        'gradient-activity': 'linear-gradient(135deg, #4ADE80 0%, #22C55E 100%)',
        'gradient-nutrition': 'linear-gradient(135deg, #F59E0B 0%, #D97706 100%)',
        'gradient-recovery': 'linear-gradient(135deg, #818CF8 0%, #6366F1 100%)',
        'gradient-mobility': 'linear-gradient(135deg, #EC4899 0%, #DB2777 100%)',
        'gradient-pierre-horizontal': 'linear-gradient(90deg, #8B5CF6 0%, #22D3EE 100%)',
      },
      animation: {
        'slide-up': 'slideUp 0.2s ease-out',
        'fade-in': 'fadeIn 0.15s ease-out',
        'scale-in': 'scaleIn 0.2s ease-out',
        'shimmer': 'shimmer 2s infinite linear',
        'pulse-glow': 'pulseGlow 2s infinite',
      },
      keyframes: {
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        scaleIn: {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
        shimmer: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        pulseGlow: {
          '0%, 100%': { boxShadow: '0 0 0 0 rgba(124, 58, 237, 0.4)' },
          '50%': { boxShadow: '0 0 0 8px rgba(124, 58, 237, 0)' },
        },
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
    require('@tailwindcss/typography'),
  ],
}