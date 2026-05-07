import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  build: {
    target: 'esnext',
    minify: 'esbuild',
    cssMinify: true,
  },
  optimizeDeps: {
    exclude: ['@tauri-apps/api'],
  },
  server: {
    port: 1428,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1429,
        }
      : undefined,
  },
});
