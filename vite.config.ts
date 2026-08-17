import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// In production todo-server serves the built assets and the API from the same
// origin. In dev Vite serves the assets, so it has to proxy the API through —
// otherwise /api/* falls through to the SPA fallback and returns index.html.
const apiTarget = process.env.TODO_SERVER_URL || "http://localhost:8080";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      "/api": { target: apiTarget, changeOrigin: true },
      "/health": { target: apiTarget, changeOrigin: true },
      // SSE — must not be buffered or timed out by the proxy
      "/events": { target: apiTarget, changeOrigin: true, ws: false, timeout: 0 },
    },
  },
});
