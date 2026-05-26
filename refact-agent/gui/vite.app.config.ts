/**
 * Standalone app build config for the Electron desktop application.
 * Produces dist/app/index.html + assets (NOT a library build).
 *
 * Build command (from refact-agent/gui/):
 *   VITE_REFACT_LSP_URL=http://127.0.0.1:8486 \
 *   VITE_UPLOAD_API_URL=http://127.0.0.1:8002 \
 *   VITE_EMBEDDED_MODE=true \
 *   vite build --config vite.app.config.ts
 *
 * Defaults: LSP HTTP on 8486 (must match craftifai-desktop spawnLSP), Python API on 8002.
 * Embedded UI panels default ON unless VITE_EMBEDDED_MODE=false.
 *
 * Auth/usage calls use VITE_CRAFTIF_API_BASE (defaults to production cloud API).
 * Override: VITE_CRAFTIF_API_BASE=https://api.example.com vite build --config vite.app.config.ts
 */
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react-swc";
import { execSync } from "child_process";

let commitHash = "desktop";
try {
  commitHash = execSync("git rev-parse --short HEAD", { encoding: "utf-8" }).trim();
} catch {
  commitHash = "desktop";
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const craftifApiBase =
    env.VITE_CRAFTIF_API_BASE || "https://api.craftifai.com";
  const lspHttpUrl =
    env.VITE_REFACT_LSP_URL || "http://127.0.0.1:8486";
  const uploadApiUrl =
    env.VITE_UPLOAD_API_URL || "http://127.0.0.1:8002";
  const embeddedDesktop =
    env.VITE_EMBEDDED_MODE === "false" ? false : true;

  return {
    plugins: [react()],
    base: "./",
    build: {
      outDir: "dist/app",
      emptyOutDir: true,
      sourcemap: false,
      rollupOptions: {
        onwarn(warning, defaultHandler) {
          if (warning.code === "SOURCEMAP_ERROR") return;
          defaultHandler(warning);
        },
      },
    },
    define: {
      "process.env.NODE_ENV": '"production"',
      "process.env.DEBUG": "undefined",
      __REFACT_CHAT_VERSION__: JSON.stringify({
        semver: process.env.npm_package_version ?? "desktop",
        commit: commitHash,
      }),
      __REFACT_LSP_PORT__: "8486",
      __REFACT_EMBEDDED_MODE__: JSON.stringify(embeddedDesktop),
      "import.meta.env.VITE_CRAFTIF_API_BASE": JSON.stringify(craftifApiBase),
      "import.meta.env.VITE_REFACT_LSP_URL": JSON.stringify(lspHttpUrl),
      "import.meta.env.VITE_UPLOAD_API_URL": JSON.stringify(uploadApiUrl),
    },
  };
});
