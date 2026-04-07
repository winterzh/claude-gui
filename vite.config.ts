import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import fs from "fs";

const host = process.env.TAURI_DEV_HOST;

// Read packaging config if exists and enabled
let packagingConfig: unknown = null;
try {
  const configPath = "./config-packaging/config.json";
  if (fs.existsSync(configPath)) {
    const raw = JSON.parse(fs.readFileSync(configPath, "utf8"));
    if (raw.enabled === true) {
      packagingConfig = raw;
    }
  }
} catch {}

export default defineConfig(async () => ({
  plugins: [react()],
  define: {
    __PACKAGING_CONFIG__: JSON.stringify(packagingConfig),
  },
  clearScreen: false,
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
      ignored: ["**/src-tauri/**"],
    },
  },
}));
