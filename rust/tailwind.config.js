/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // Apple-inspired color palette
        'apple-gray': {
          50: '#fafafa',
          100: '#f5f5f7',
          200: '#e8e8ed',
          300: '#d2d2d7',
          400: '#86868b',
          500: '#6e6e73',
          600: '#1d1d1f',
        },
        'apple-blue': {
          DEFAULT: '#007aff',
          hover: '#0066d6',
        },
        'apple-green': '#34c759',
        'apple-red': '#ff3b30',
        'apple-orange': '#ff9500',
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          'SF Pro Display',
          'SF Pro Text',
          'Helvetica Neue',
          'Arial',
          'sans-serif',
        ],
        mono: ['SF Mono', 'Monaco', 'Consolas', 'monospace'],
      },
      borderRadius: {
        'apple': '12px',
        'apple-lg': '16px',
        'apple-xl': '20px',
      },
      boxShadow: {
        'apple': '0 4px 16px rgba(0, 0, 0, 0.08)',
        'apple-lg': '0 8px 32px rgba(0, 0, 0, 0.12)',
        'apple-inset': 'inset 0 0 0 1px rgba(0, 0, 0, 0.1)',
      },
      backdropBlur: {
        'apple': '20px',
      },
    },
  },
  plugins: [],
}
