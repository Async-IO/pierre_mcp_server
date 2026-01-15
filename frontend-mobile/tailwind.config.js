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
          // Three Pillars
          activity: '#10B981',
          nutrition: '#F59E0B',
          recovery: '#6366F1',
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
          primary: '#0f0f0f',
          secondary: '#1a1a1a',
          tertiary: '#2a2a2a',
          elevated: '#333333',
        },
        text: {
          primary: '#ffffff',
          secondary: '#a1a1a1',
          tertiary: '#6b6b6b',
          accent: '#7C3AED',
        },
        border: {
          subtle: '#2a2a2a',
          default: '#3a3a3a',
          strong: '#4a4a4a',
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
        sans: ['System', 'sans-serif'],
        mono: ['Menlo', 'monospace'],
      },
    },
  },
  plugins: [],
};
