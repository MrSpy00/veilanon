// @ts-nocheck
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process?.env?.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],
  // SECURITY: Only public Vite prefixes are exposed to the browser bundle.
  // VEILANON_* secrets MUST NOT be exposed via Vite (they are resolved
  // in Rust via config::var > secrets.enc > XOR-obfuscated embed). Including
  // VEILANON_ here would inline secrets like LIVEKIT_API_SECRET into JS.
  envPrefix: ['VITE_', 'PUBLIC_'],

  build: {
    chunkSizeWarningLimit: 850,
    rollupOptions: {
      output: {
        manualChunks: (id /*: string*/) => {
          if (id.includes('node_modules/livekit-client')) return 'livekit';
          if (id.includes('node_modules/@tauri-apps')) return 'tauri';
          if (id.includes('node_modules/svelte')) return 'svelte';
          if (id.includes('node_modules/@mediapipe')) return 'mediapipe';
          if (id.includes('src/lib/components/settings')) return 'settings';
          if (id.includes('src/lib/components/chat')) return 'chat';
          if (id.includes('src/lib/components/layout')) return 'layout';
          if (id.includes('src/lib/components/spaces')) return 'spaces';
          if (id.includes('src/lib/components/social')) return 'social';
          if (id.includes('src/lib/components/media')) return 'media';
          if (id.includes('src/lib/stores')) return 'stores';
          return undefined;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
