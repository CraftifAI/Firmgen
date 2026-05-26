/// <reference types="vitest" />
import path from "path";
import { PluginOption, UserConfig, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";

import { coverageConfigDefaults } from "vitest/config";
import dts from "vite-plugin-dts";

import { execSync } from "child_process";

let commitHash = "unknown";
try {
  commitHash = execSync("git rev-parse --short HEAD", { encoding: "utf-8" }).trim();
} catch {
  // Git not available or not in a git repo, use fallback
  commitHash = "dev";
}

// TODO: remove extra compile step when vscode can run esmodules  https://github.com/microsoft/vscode/issues/130367

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
function makeConfig(library: "browser" | "node") {
  return defineConfig(({ command, mode }) => {
    const OUT_DIR = library === "browser" ? "dist/chat" : "dist/events";
    const CONFIG: UserConfig = {
      // Build the webpage
      define: {
        "process.env.NODE_ENV": JSON.stringify(mode),
        __REFACT_CHAT_VERSION__: JSON.stringify({
          semver: process.env.npm_package_version,
          commit: commitHash,
        }),
        "process.env.DEBUG": JSON.stringify(process.env.DEBUG),
        __REFACT_LSP_PORT__: process.env.REFACT_LSP_PORT,
        __REFACT_EMBEDDED_MODE__: JSON.stringify(
          process.env.VITE_EMBEDDED_MODE === "true",
        ),
        // Empty in dev so /auth and /usage use Vite proxy; override via env if needed.
        "import.meta.env.VITE_CRAFTIF_API_BASE": JSON.stringify(
          process.env.VITE_CRAFTIF_API_BASE ?? "",
        ),
      },
      mode,
      build: {
        emptyOutDir: true,
        outDir: OUT_DIR,
        copyPublicDir: false,
        sourcemap: library === "browser",
        rollupOptions: {
          // TODO: remove when this issue is closed https://github.com/vitejs/vite/issues/15012
          onwarn(warning, defaultHandler) {
            if (warning.code === "SOURCEMAP_ERROR") {
              return;
            }

            defaultHandler(warning);
          },
        },
      },
      plugins: [react()],
      server: {
        proxy: {
          "/v1": process.env.REFACT_LSP_URL ?? "http://127.0.0.1:8001",
          "/auth": {
            target: "https://api.craftifai.com",
            changeOrigin: true,
            secure: false,
          },
          "/usage": {
            target: "https://api.craftifai.com",
            changeOrigin: true,
            secure: false,
          },
        },
      },
      test: {
        retry: 2,
        environment: "happy-dom",
        coverage: {
          exclude: coverageConfigDefaults.exclude.concat(
            "**/*.stories.@(js|jsx|mjs|ts|tsx)",
          ),
        },
        setupFiles: ["./src/utils/test-setup.ts"],
      },
      css: {
        modules: {},
      },
    };

    if (command !== "serve") {
      CONFIG.mode = "production";
      CONFIG.define = {
        ...CONFIG.define,
        "process.env.NODE_ENV": "'production'",
      };

      CONFIG.plugins?.push([
        // eslint-disable-next-line @typescript-eslint/no-unsafe-call
        eslint() as PluginOption,
      ]);

      CONFIG.plugins?.push([
        dts({
          outDir: OUT_DIR,
          rollupTypes: true,
          insertTypesEntry: true,
        }),
      ]);

      CONFIG.build = {
        ...CONFIG.build,
        lib: {
          entry:
            library === "browser"
              ? path.resolve(__dirname, "src/lib/index.ts")
              : path.resolve(__dirname, "src/events/index.ts"),
          name: "RefactChat",
          fileName: "index",
        },
      };
    }

    return CONFIG;
  });
}

export default makeConfig("browser");

export const nodeConfig = makeConfig("node");
