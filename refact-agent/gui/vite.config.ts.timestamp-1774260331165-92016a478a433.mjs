// vite.config.ts
import path from "path";
import { defineConfig } from "file:///home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui/node_modules/vite/dist/node/index.js";
import react from "file:///home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui/node_modules/@vitejs/plugin-react-swc/index.mjs";
import eslint from "file:///home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui/node_modules/vite-plugin-eslint/dist/index.mjs";
import { coverageConfigDefaults } from "file:///home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui/node_modules/vitest/dist/config.js";
import dts from "file:///home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui/node_modules/vite-plugin-dts/dist/index.mjs";
import { execSync } from "child_process";
var __vite_injected_original_dirname = "/home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/gui";
var commitHash = "unknown";
try {
  commitHash = execSync("git rev-parse --short HEAD", { encoding: "utf-8" }).trim();
} catch {
  commitHash = "dev";
}
function makeConfig(library) {
  return defineConfig(({ command, mode }) => {
    const OUT_DIR = library === "browser" ? "dist/chat" : "dist/events";
    const CONFIG = {
      // Build the webpage
      define: {
        "process.env.NODE_ENV": JSON.stringify(mode),
        __REFACT_CHAT_VERSION__: JSON.stringify({
          semver: process.env.npm_package_version,
          commit: commitHash
        }),
        "process.env.DEBUG": JSON.stringify(process.env.DEBUG),
        __REFACT_LSP_PORT__: process.env.REFACT_LSP_PORT,
        __REFACT_EMBEDDED_MODE__: JSON.stringify(
          process.env.VITE_EMBEDDED_MODE === "true"
        )
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
          }
        }
      },
      plugins: [react()],
      server: {
        proxy: {
          "/v1": process.env.REFACT_LSP_URL ?? "http://127.0.0.1:8001"
        }
      },
      test: {
        retry: 2,
        environment: "happy-dom",
        coverage: {
          exclude: coverageConfigDefaults.exclude.concat(
            "**/*.stories.@(js|jsx|mjs|ts|tsx)"
          )
        },
        setupFiles: ["./src/utils/test-setup.ts"]
      },
      css: {
        modules: {}
      }
    };
    if (command !== "serve") {
      CONFIG.mode = "production";
      CONFIG.define = {
        ...CONFIG.define,
        "process.env.NODE_ENV": "'production'"
      };
      CONFIG.plugins?.push([
        // eslint-disable-next-line @typescript-eslint/no-unsafe-call
        eslint()
      ]);
      CONFIG.plugins?.push([
        dts({
          outDir: OUT_DIR,
          rollupTypes: true,
          insertTypesEntry: true
        })
      ]);
      CONFIG.build = {
        ...CONFIG.build,
        lib: {
          entry: library === "browser" ? path.resolve(__vite_injected_original_dirname, "src/lib/index.ts") : path.resolve(__vite_injected_original_dirname, "src/events/index.ts"),
          name: "RefactChat",
          fileName: "index"
        }
      };
    }
    return CONFIG;
  });
}
var vite_config_default = makeConfig("browser");
var nodeConfig = makeConfig("node");
export {
  vite_config_default as default,
  nodeConfig
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvaG9tZS9yaXRpay9DcmFmdGlmL0NSQUZUSUYvSUlTX2FnZW50LW1haW4vcmVmYWN0LWFnZW50L2d1aVwiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9maWxlbmFtZSA9IFwiL2hvbWUvcml0aWsvQ3JhZnRpZi9DUkFGVElGL0lJU19hZ2VudC1tYWluL3JlZmFjdC1hZ2VudC9ndWkvdml0ZS5jb25maWcudHNcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfaW1wb3J0X21ldGFfdXJsID0gXCJmaWxlOi8vL2hvbWUvcml0aWsvQ3JhZnRpZi9DUkFGVElGL0lJU19hZ2VudC1tYWluL3JlZmFjdC1hZ2VudC9ndWkvdml0ZS5jb25maWcudHNcIjsvLy8gPHJlZmVyZW5jZSB0eXBlcz1cInZpdGVzdFwiIC8+XG5pbXBvcnQgcGF0aCBmcm9tIFwicGF0aFwiO1xuaW1wb3J0IHsgUGx1Z2luT3B0aW9uLCBVc2VyQ29uZmlnLCBkZWZpbmVDb25maWcgfSBmcm9tIFwidml0ZVwiO1xuaW1wb3J0IHJlYWN0IGZyb20gXCJAdml0ZWpzL3BsdWdpbi1yZWFjdC1zd2NcIjtcbmltcG9ydCBlc2xpbnQgZnJvbSBcInZpdGUtcGx1Z2luLWVzbGludFwiO1xuXG5pbXBvcnQgeyBjb3ZlcmFnZUNvbmZpZ0RlZmF1bHRzIH0gZnJvbSBcInZpdGVzdC9jb25maWdcIjtcbmltcG9ydCBkdHMgZnJvbSBcInZpdGUtcGx1Z2luLWR0c1wiO1xuXG5pbXBvcnQgeyBleGVjU3luYyB9IGZyb20gXCJjaGlsZF9wcm9jZXNzXCI7XG5cbmxldCBjb21taXRIYXNoID0gXCJ1bmtub3duXCI7XG50cnkge1xuICBjb21taXRIYXNoID0gZXhlY1N5bmMoXCJnaXQgcmV2LXBhcnNlIC0tc2hvcnQgSEVBRFwiLCB7IGVuY29kaW5nOiBcInV0Zi04XCIgfSkudHJpbSgpO1xufSBjYXRjaCB7XG4gIC8vIEdpdCBub3QgYXZhaWxhYmxlIG9yIG5vdCBpbiBhIGdpdCByZXBvLCB1c2UgZmFsbGJhY2tcbiAgY29tbWl0SGFzaCA9IFwiZGV2XCI7XG59XG5cbi8vIFRPRE86IHJlbW92ZSBleHRyYSBjb21waWxlIHN0ZXAgd2hlbiB2c2NvZGUgY2FuIHJ1biBlc21vZHVsZXMgIGh0dHBzOi8vZ2l0aHViLmNvbS9taWNyb3NvZnQvdnNjb2RlL2lzc3Vlcy8xMzAzNjdcblxuLy8gaHR0cHM6Ly92aXRlanMuZGV2L2NvbmZpZy9cbi8qKiBAdHlwZSB7aW1wb3J0KCd2aXRlJykuVXNlckNvbmZpZ30gKi9cbmZ1bmN0aW9uIG1ha2VDb25maWcobGlicmFyeTogXCJicm93c2VyXCIgfCBcIm5vZGVcIikge1xuICByZXR1cm4gZGVmaW5lQ29uZmlnKCh7IGNvbW1hbmQsIG1vZGUgfSkgPT4ge1xuICAgIGNvbnN0IE9VVF9ESVIgPSBsaWJyYXJ5ID09PSBcImJyb3dzZXJcIiA/IFwiZGlzdC9jaGF0XCIgOiBcImRpc3QvZXZlbnRzXCI7XG4gICAgY29uc3QgQ09ORklHOiBVc2VyQ29uZmlnID0ge1xuICAgICAgLy8gQnVpbGQgdGhlIHdlYnBhZ2VcbiAgICAgIGRlZmluZToge1xuICAgICAgICBcInByb2Nlc3MuZW52Lk5PREVfRU5WXCI6IEpTT04uc3RyaW5naWZ5KG1vZGUpLFxuICAgICAgICBfX1JFRkFDVF9DSEFUX1ZFUlNJT05fXzogSlNPTi5zdHJpbmdpZnkoe1xuICAgICAgICAgIHNlbXZlcjogcHJvY2Vzcy5lbnYubnBtX3BhY2thZ2VfdmVyc2lvbixcbiAgICAgICAgICBjb21taXQ6IGNvbW1pdEhhc2gsXG4gICAgICAgIH0pLFxuICAgICAgICBcInByb2Nlc3MuZW52LkRFQlVHXCI6IEpTT04uc3RyaW5naWZ5KHByb2Nlc3MuZW52LkRFQlVHKSxcbiAgICAgICAgX19SRUZBQ1RfTFNQX1BPUlRfXzogcHJvY2Vzcy5lbnYuUkVGQUNUX0xTUF9QT1JULFxuICAgICAgICBfX1JFRkFDVF9FTUJFRERFRF9NT0RFX186IEpTT04uc3RyaW5naWZ5KFxuICAgICAgICAgIHByb2Nlc3MuZW52LlZJVEVfRU1CRURERURfTU9ERSA9PT0gXCJ0cnVlXCIsXG4gICAgICAgICksXG4gICAgICB9LFxuICAgICAgbW9kZSxcbiAgICAgIGJ1aWxkOiB7XG4gICAgICAgIGVtcHR5T3V0RGlyOiB0cnVlLFxuICAgICAgICBvdXREaXI6IE9VVF9ESVIsXG4gICAgICAgIGNvcHlQdWJsaWNEaXI6IGZhbHNlLFxuICAgICAgICBzb3VyY2VtYXA6IGxpYnJhcnkgPT09IFwiYnJvd3NlclwiLFxuICAgICAgICByb2xsdXBPcHRpb25zOiB7XG4gICAgICAgICAgLy8gVE9ETzogcmVtb3ZlIHdoZW4gdGhpcyBpc3N1ZSBpcyBjbG9zZWQgaHR0cHM6Ly9naXRodWIuY29tL3ZpdGVqcy92aXRlL2lzc3Vlcy8xNTAxMlxuICAgICAgICAgIG9ud2Fybih3YXJuaW5nLCBkZWZhdWx0SGFuZGxlcikge1xuICAgICAgICAgICAgaWYgKHdhcm5pbmcuY29kZSA9PT0gXCJTT1VSQ0VNQVBfRVJST1JcIikge1xuICAgICAgICAgICAgICByZXR1cm47XG4gICAgICAgICAgICB9XG5cbiAgICAgICAgICAgIGRlZmF1bHRIYW5kbGVyKHdhcm5pbmcpO1xuICAgICAgICAgIH0sXG4gICAgICAgIH0sXG4gICAgICB9LFxuICAgICAgcGx1Z2luczogW3JlYWN0KCldLFxuICAgICAgc2VydmVyOiB7XG4gICAgICAgIHByb3h5OiB7XG4gICAgICAgICAgXCIvdjFcIjogcHJvY2Vzcy5lbnYuUkVGQUNUX0xTUF9VUkwgPz8gXCJodHRwOi8vMTI3LjAuMC4xOjgwMDFcIixcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgICB0ZXN0OiB7XG4gICAgICAgIHJldHJ5OiAyLFxuICAgICAgICBlbnZpcm9ubWVudDogXCJoYXBweS1kb21cIixcbiAgICAgICAgY292ZXJhZ2U6IHtcbiAgICAgICAgICBleGNsdWRlOiBjb3ZlcmFnZUNvbmZpZ0RlZmF1bHRzLmV4Y2x1ZGUuY29uY2F0KFxuICAgICAgICAgICAgXCIqKi8qLnN0b3JpZXMuQChqc3xqc3h8bWpzfHRzfHRzeClcIixcbiAgICAgICAgICApLFxuICAgICAgICB9LFxuICAgICAgICBzZXR1cEZpbGVzOiBbXCIuL3NyYy91dGlscy90ZXN0LXNldHVwLnRzXCJdLFxuICAgICAgfSxcbiAgICAgIGNzczoge1xuICAgICAgICBtb2R1bGVzOiB7fSxcbiAgICAgIH0sXG4gICAgfTtcblxuICAgIGlmIChjb21tYW5kICE9PSBcInNlcnZlXCIpIHtcbiAgICAgIENPTkZJRy5tb2RlID0gXCJwcm9kdWN0aW9uXCI7XG4gICAgICBDT05GSUcuZGVmaW5lID0ge1xuICAgICAgICAuLi5DT05GSUcuZGVmaW5lLFxuICAgICAgICBcInByb2Nlc3MuZW52Lk5PREVfRU5WXCI6IFwiJ3Byb2R1Y3Rpb24nXCIsXG4gICAgICB9O1xuXG4gICAgICBDT05GSUcucGx1Z2lucz8ucHVzaChbXG4gICAgICAgIC8vIGVzbGludC1kaXNhYmxlLW5leHQtbGluZSBAdHlwZXNjcmlwdC1lc2xpbnQvbm8tdW5zYWZlLWNhbGxcbiAgICAgICAgZXNsaW50KCkgYXMgUGx1Z2luT3B0aW9uLFxuICAgICAgXSk7XG5cbiAgICAgIENPTkZJRy5wbHVnaW5zPy5wdXNoKFtcbiAgICAgICAgZHRzKHtcbiAgICAgICAgICBvdXREaXI6IE9VVF9ESVIsXG4gICAgICAgICAgcm9sbHVwVHlwZXM6IHRydWUsXG4gICAgICAgICAgaW5zZXJ0VHlwZXNFbnRyeTogdHJ1ZSxcbiAgICAgICAgfSksXG4gICAgICBdKTtcblxuICAgICAgQ09ORklHLmJ1aWxkID0ge1xuICAgICAgICAuLi5DT05GSUcuYnVpbGQsXG4gICAgICAgIGxpYjoge1xuICAgICAgICAgIGVudHJ5OlxuICAgICAgICAgICAgbGlicmFyeSA9PT0gXCJicm93c2VyXCJcbiAgICAgICAgICAgICAgPyBwYXRoLnJlc29sdmUoX19kaXJuYW1lLCBcInNyYy9saWIvaW5kZXgudHNcIilcbiAgICAgICAgICAgICAgOiBwYXRoLnJlc29sdmUoX19kaXJuYW1lLCBcInNyYy9ldmVudHMvaW5kZXgudHNcIiksXG4gICAgICAgICAgbmFtZTogXCJSZWZhY3RDaGF0XCIsXG4gICAgICAgICAgZmlsZU5hbWU6IFwiaW5kZXhcIixcbiAgICAgICAgfSxcbiAgICAgIH07XG4gICAgfVxuXG4gICAgcmV0dXJuIENPTkZJRztcbiAgfSk7XG59XG5cbmV4cG9ydCBkZWZhdWx0IG1ha2VDb25maWcoXCJicm93c2VyXCIpO1xuXG5leHBvcnQgY29uc3Qgbm9kZUNvbmZpZyA9IG1ha2VDb25maWcoXCJub2RlXCIpO1xuIl0sCiAgIm1hcHBpbmdzIjogIjtBQUNBLE9BQU8sVUFBVTtBQUNqQixTQUFtQyxvQkFBb0I7QUFDdkQsT0FBTyxXQUFXO0FBQ2xCLE9BQU8sWUFBWTtBQUVuQixTQUFTLDhCQUE4QjtBQUN2QyxPQUFPLFNBQVM7QUFFaEIsU0FBUyxnQkFBZ0I7QUFUekIsSUFBTSxtQ0FBbUM7QUFXekMsSUFBSSxhQUFhO0FBQ2pCLElBQUk7QUFDRixlQUFhLFNBQVMsOEJBQThCLEVBQUUsVUFBVSxRQUFRLENBQUMsRUFBRSxLQUFLO0FBQ2xGLFFBQVE7QUFFTixlQUFhO0FBQ2Y7QUFNQSxTQUFTLFdBQVcsU0FBNkI7QUFDL0MsU0FBTyxhQUFhLENBQUMsRUFBRSxTQUFTLEtBQUssTUFBTTtBQUN6QyxVQUFNLFVBQVUsWUFBWSxZQUFZLGNBQWM7QUFDdEQsVUFBTSxTQUFxQjtBQUFBO0FBQUEsTUFFekIsUUFBUTtBQUFBLFFBQ04sd0JBQXdCLEtBQUssVUFBVSxJQUFJO0FBQUEsUUFDM0MseUJBQXlCLEtBQUssVUFBVTtBQUFBLFVBQ3RDLFFBQVEsUUFBUSxJQUFJO0FBQUEsVUFDcEIsUUFBUTtBQUFBLFFBQ1YsQ0FBQztBQUFBLFFBQ0QscUJBQXFCLEtBQUssVUFBVSxRQUFRLElBQUksS0FBSztBQUFBLFFBQ3JELHFCQUFxQixRQUFRLElBQUk7QUFBQSxRQUNqQywwQkFBMEIsS0FBSztBQUFBLFVBQzdCLFFBQVEsSUFBSSx1QkFBdUI7QUFBQSxRQUNyQztBQUFBLE1BQ0Y7QUFBQSxNQUNBO0FBQUEsTUFDQSxPQUFPO0FBQUEsUUFDTCxhQUFhO0FBQUEsUUFDYixRQUFRO0FBQUEsUUFDUixlQUFlO0FBQUEsUUFDZixXQUFXLFlBQVk7QUFBQSxRQUN2QixlQUFlO0FBQUE7QUFBQSxVQUViLE9BQU8sU0FBUyxnQkFBZ0I7QUFDOUIsZ0JBQUksUUFBUSxTQUFTLG1CQUFtQjtBQUN0QztBQUFBLFlBQ0Y7QUFFQSwyQkFBZSxPQUFPO0FBQUEsVUFDeEI7QUFBQSxRQUNGO0FBQUEsTUFDRjtBQUFBLE1BQ0EsU0FBUyxDQUFDLE1BQU0sQ0FBQztBQUFBLE1BQ2pCLFFBQVE7QUFBQSxRQUNOLE9BQU87QUFBQSxVQUNMLE9BQU8sUUFBUSxJQUFJLGtCQUFrQjtBQUFBLFFBQ3ZDO0FBQUEsTUFDRjtBQUFBLE1BQ0EsTUFBTTtBQUFBLFFBQ0osT0FBTztBQUFBLFFBQ1AsYUFBYTtBQUFBLFFBQ2IsVUFBVTtBQUFBLFVBQ1IsU0FBUyx1QkFBdUIsUUFBUTtBQUFBLFlBQ3RDO0FBQUEsVUFDRjtBQUFBLFFBQ0Y7QUFBQSxRQUNBLFlBQVksQ0FBQywyQkFBMkI7QUFBQSxNQUMxQztBQUFBLE1BQ0EsS0FBSztBQUFBLFFBQ0gsU0FBUyxDQUFDO0FBQUEsTUFDWjtBQUFBLElBQ0Y7QUFFQSxRQUFJLFlBQVksU0FBUztBQUN2QixhQUFPLE9BQU87QUFDZCxhQUFPLFNBQVM7QUFBQSxRQUNkLEdBQUcsT0FBTztBQUFBLFFBQ1Ysd0JBQXdCO0FBQUEsTUFDMUI7QUFFQSxhQUFPLFNBQVMsS0FBSztBQUFBO0FBQUEsUUFFbkIsT0FBTztBQUFBLE1BQ1QsQ0FBQztBQUVELGFBQU8sU0FBUyxLQUFLO0FBQUEsUUFDbkIsSUFBSTtBQUFBLFVBQ0YsUUFBUTtBQUFBLFVBQ1IsYUFBYTtBQUFBLFVBQ2Isa0JBQWtCO0FBQUEsUUFDcEIsQ0FBQztBQUFBLE1BQ0gsQ0FBQztBQUVELGFBQU8sUUFBUTtBQUFBLFFBQ2IsR0FBRyxPQUFPO0FBQUEsUUFDVixLQUFLO0FBQUEsVUFDSCxPQUNFLFlBQVksWUFDUixLQUFLLFFBQVEsa0NBQVcsa0JBQWtCLElBQzFDLEtBQUssUUFBUSxrQ0FBVyxxQkFBcUI7QUFBQSxVQUNuRCxNQUFNO0FBQUEsVUFDTixVQUFVO0FBQUEsUUFDWjtBQUFBLE1BQ0Y7QUFBQSxJQUNGO0FBRUEsV0FBTztBQUFBLEVBQ1QsQ0FBQztBQUNIO0FBRUEsSUFBTyxzQkFBUSxXQUFXLFNBQVM7QUFFNUIsSUFBTSxhQUFhLFdBQVcsTUFBTTsiLAogICJuYW1lcyI6IFtdCn0K
