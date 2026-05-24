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

// Single source of truth for the displayed version: package.json.
const appVersion = JSON.parse(fs.readFileSync("./package.json", "utf8")).version;

export default defineConfig(async () => ({
  plugins: [react()],
  define: {
    __PACKAGING_CONFIG__: JSON.stringify(packagingConfig),
    __APP_VERSION__: JSON.stringify(appVersion),
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
