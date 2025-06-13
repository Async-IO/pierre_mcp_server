/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'api-blue': '#2563eb',
        'api-green': '#059669',
        'api-red': '#dc2626',
        'api-yellow': '#d97706',
      }
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}