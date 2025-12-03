// ABOUTME: Tailwind configuration with Pierre design system colors
// ABOUTME: Standard Tailwind config with Pierre brand palette

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        pierre: {
          violet: '#7C3AED',
          cyan: '#06B6D4',
          activity: '#10B981',
          nutrition: '#F59E0B',
          recovery: '#6366F1',
          dark: '#0F0F1A',
          slate: '#1E1E2E',
          'violet-light': '#A78BFA',
          'violet-dark': '#5B21B6',
          'cyan-light': '#22D3EE',
          'activity-dark': '#059669',
          'nutrition-dark': '#D97706',
          'recovery-dark': '#4F46E5',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Monaco', 'Menlo', 'monospace'],
      },
    },
  },
  plugins: [],
};
