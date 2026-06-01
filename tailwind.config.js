/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: { extend: { fontFamily: { sans: ['-apple-system','BlinkMacSystemFont','SF Pro Text','Segoe UI','sans-serif'], mono: ['SFMono-Regular','ui-monospace','Menlo','monospace'] } } },
  plugins: [],
};
