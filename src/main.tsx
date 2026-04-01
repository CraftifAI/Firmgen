/**
 * Entry for Vite (dev) and desktop bundle (vite.app.config.ts).
 */

import { render } from "./lib";

const element = document.getElementById("refact-chat");

if (element) {
  // LSP HTTP (refact-lsp --http-port); desktop build sets this to 8486 via vite.app.config.ts
  const lspUrl =
    import.meta.env.VITE_REFACT_LSP_URL || "http://127.0.0.1:8486";
  const portMatch = lspUrl.match(/:(\d+)/);
  const lspPort = portMatch ? parseInt(portMatch[1], 10) : 8486;
  // File upload/parse (refactapi). Dockerfile.api uses port 8002; local uvicorn often 8000.
  const uploadApiUrl =
    import.meta.env.VITE_UPLOAD_API_URL || "http://127.0.0.1:8002";

  console.log("CraftifAI GUI: Connecting to agent on port", lspPort, "from URL:", lspUrl);

  // Check for embedded mode from environment variable
  const embeddedMode = (__REFACT_EMBEDDED_MODE__ as boolean) ?? false;
  console.log("Embedded mode enabled:", embeddedMode, "from __REFACT_EMBEDDED_MODE__:", __REFACT_EMBEDDED_MODE__);

  render(element, {
    host: "web",
    features: {
      statistics: true,
      vecdb: true,
      ast: true,
      embedded: embeddedMode,
    },
    themeProps: {},
    lspPort: lspPort,
    lspUrl,
    uploadApiUrl,
  });
}
