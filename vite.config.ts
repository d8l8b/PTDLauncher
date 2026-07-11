import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Prevent Vite from obscuring Rust errors
  clearScreen: false,

  // Development server tailored for Tauri
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    optimizeDeps: {
      esbuildOptions: {
        // This helps if the native binary is failing to communicate
        external: ["@esbuild/linux-x64"],
      },
    },
  },

  build: {
    target: 'es2020',
    sourcemap: process.env.ANALYZE === 'true',
    brotliSize: true,
    cssCodeSplit: true,
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            if (id.includes('react')) return 'vendor_react';
            return 'vendor';
          }
        },
      },
    },
  },
}));
