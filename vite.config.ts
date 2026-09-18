import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import pkg from "./package.json";

// @ts-expect-error type error without @types/node package
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react()],
  clearScreen: false,
  // Expose the app version (from package.json) to the frontend as a compile-time constant.
  define: { __APP_VERSION__: JSON.stringify(pkg.version) },
  server: {
    port: 1420,
    strictPort: false,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
