import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      // Proxy API requests to Rust backend
      '/lobby': 'http://localhost:8080',
      '/kanji': 'http://localhost:8080',
      '/check_word': 'http://localhost:8080'
    }
  }
})
