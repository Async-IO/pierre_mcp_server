// ABOUTME: Tailwind CSS configuration for NativeWind v4
// ABOUTME: Defines theme colors matching Pierre design system and content paths

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./App.{js,jsx,ts,tsx}",
    "./src/**/*.{js,jsx,ts,tsx}",
  ],
  presets: [require('nativewind/preset')],
  theme: {
    extend: {
      colors: {
        // Pierre Design System - Brand Colors
        pierre: {
          violet: '#7C3AED',
          cyan: '#06B6D4',
          // Three Pillars (dark-mode optimized)
          activity: '#10B981',
          nutrition: '#FBBF24',  // Brightened for dark mode contrast
          recovery: '#818CF8',   // Brightened for dark mode contrast
          // Dark theme
          dark: '#0F0F1A',
          slate: '#1E1E2E',
          // Extended palettes
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
          },
          yellow: {
            50: '#fefce8',
            100: '#fef3c7',
            500: '#eab308',
            600: '#ca8a04',
            700: '#a16207',
          },
          red: {
            50: '#fef2f2',
            100: '#fee2e2',
            500: '#ef4444',
            600: '#dc2626',
            700: '#b91c1c',
          },
          blue: {
            50: '#eff6ff',
            100: '#dbeafe',
            500: '#3b82f6',
            600: '#2563eb',
            700: '#1d4ed8',
          },
        },
        // Primary brand color (sky blue palette)
        primary: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          200: '#bae6fd',
          300: '#7dd3fc',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          800: '#075985',
          900: '#0c4a6e',
          950: '#082f49',
        },
        background: {
          primary: '#0F0F1A',    // pierre-dark - deepest background
          secondary: '#1E1E2E',  // pierre-slate - cards, elevated surfaces
          tertiary: '#2A2A3E',   // slightly lighter for hover states
          elevated: '#363650',   // elevated components like modals
        },
        text: {
          primary: '#ffffff',
          secondary: '#a1a1aa',  // zinc-400
          tertiary: '#71717a',   // zinc-500
          accent: '#7C3AED',     // pierre-violet
        },
        border: {
          subtle: 'rgba(255, 255, 255, 0.05)',   // white/5
          default: 'rgba(255, 255, 255, 0.1)',   // white/10
          strong: 'rgba(255, 255, 255, 0.15)',   // white/15
        },
        success: '#22c55e',
        warning: '#f59e0b',
        error: '#ef4444',
        // Provider brand colors
        providers: {
          strava: '#FC4C02',
          garmin: '#007CC3',
          fitbit: '#00B0B9',
          whoop: '#00D46A',
          terra: '#6366F1',
        },
      },
      fontFamily: {
        // For React Native, use custom fonts loaded via expo-font
        // Plus Jakarta Sans or Satoshi recommended for premium feel
        sans: ['PlusJakartaSans', 'System', 'sans-serif'],
        mono: ['JetBrainsMono', 'Menlo', 'monospace'],
      },
      boxShadow: {
        // Glow effects for premium interactions
        'glow-violet': '0 0 20px rgba(124, 58, 237, 0.3)',
        'glow-cyan': '0 0 20px rgba(6, 182, 212, 0.3)',
        'glow-activity': '0 0 20px rgba(16, 185, 129, 0.3)',
        'glow-nutrition': '0 0 20px rgba(251, 191, 36, 0.3)',
        'glow-recovery': '0 0 20px rgba(129, 140, 248, 0.3)',
      },
    },
  },
  plugins: [],
};
