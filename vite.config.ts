import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    watch: { ignored: ["**/src-tauri/**", "**/legacy/**", "**/website/**"] },
  },
  build: {
    target: "es2022",
    cssMinify: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
        chat: resolve(__dirname, "chat.html"),
      },
    },
  },
});
