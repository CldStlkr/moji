import type { Config } from 'tailwindcss'

export default {
  darkMode: 'class',
  content: [
    "./src/**/*.rs",
    "./index.html",
    "./**/*.html"
  ],
} satisfies Config
