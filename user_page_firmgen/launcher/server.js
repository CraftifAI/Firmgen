const express = require("express");
const cors = require("cors");
const { spawn } = require("child_process");
const path = require("path");
const net = require("net");

const app = express();
app.use(cors());
app.use(express.json());

let engineProcess = null;
let engineConfig = null;
let engineState = {
  status: "idle", // idle | starting | ready | failed | stopped
  pid: null,
  lspUrl: null,
  error: null,
};

function isPortOpen(host, port, timeout = 500) {
  return new Promise((resolve) => {
    const socket = new net.Socket();

    const done = (result) => {
      socket.destroy();
      resolve(result);
    };

    socket.setTimeout(timeout);
    socket.once("connect", () => done(true));
    socket.once("timeout", () => done(false));
    socket.once("error", () => done(false));

    socket.connect(port, host);
  });
}

async function waitForPort(host, port, retries = 40, delayMs = 500) {
  for (let i = 0; i < retries; i++) {
    const ok = await isPortOpen(host, port);
    if (ok) return true;
    await new Promise((r) => setTimeout(r, delayMs));
  }
  return false;
}

function stopEngineInternal() {
  if (engineProcess) {
    try {
      engineProcess.kill("SIGTERM");
    } catch (_) {}
    engineProcess = null;
  }

  engineState = {
    status: "stopped",
    pid: null,
    lspUrl: null,
    error: null,
  };
}

app.get("/v1/launcher/status", (_req, res) => {
  res.json({
    ok: true,
    ...engineState,
    config: engineConfig,
  });
});

app.post("/v1/launcher/stop", (_req, res) => {
  stopEngineInternal();
  res.json({ ok: true, status: engineState.status });
});

app.post("/v1/launcher/start", async (req, res) => {
  try {
    const {
      binaryPath,
      addressUrl,
      apiKey,
      vecDbPath,
      espIdfPath,
      boardDefinition,
      platform,
      workspacePath,
      httpPort = 8486,
      extraArgs = [],
    } = req.body || {};

    if (!binaryPath) {
      return res.status(400).json({ ok: false, error: "binaryPath is required" });
    }
    if (!addressUrl) {
      return res.status(400).json({ ok: false, error: "addressUrl is required" });
    }
    if (!workspacePath) {
      return res.status(400).json({ ok: false, error: "workspacePath is required" });
    }
    if (!boardDefinition) {
      return res.status(400).json({ ok: false, error: "boardDefinition is required" });
    }
    if (!platform) {
      return res.status(400).json({ ok: false, error: "platform is required" });
    }

    if (engineProcess) {
      stopEngineInternal();
    }

    const args = [
      "--address-url",
      addressUrl,
      "--http-port",
      String(httpPort),
      "--platform",
      platform,
      "--board-definition",
      boardDefinition,
      "--workspace-folder",
      workspacePath,
      "--static-vecdb",
      vecDbPath,
      "--logs-stderr",
    ];

    if (apiKey) {
      args.push("--api-key", apiKey);
    }
    if (vecDbPath) {
      args.push("--static-vecdb", vecDbPath);
    }

    if (Array.isArray(extraArgs) && extraArgs.length > 0) {
      args.push(...extraArgs);
    }

    const env = {
      ...process.env,
      ...(espIdfPath ? { IDF_PATH: espIdfPath } : {}),
    };

    engineConfig = {
      binaryPath,
      addressUrl,
      vecDbPath,
      espIdfPath,
      boardDefinition,
      platform,
      workspacePath,
      httpPort,
    };

    engineState = {
      status: "starting",
      pid: null,
      lspUrl: `http://127.0.0.1:${httpPort}`,
      error: null,
    };

    engineProcess = spawn(binaryPath, args, {
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    engineState.pid = engineProcess.pid ?? null;

    engineProcess.stdout.on("data", (chunk) => {
      process.stdout.write(`[engine stdout] ${chunk}`);
    });

    engineProcess.stderr.on("data", (chunk) => {
      process.stderr.write(`[engine stderr] ${chunk}`);
    });

    engineProcess.on("exit", (code, signal) => {
      engineState = {
        status: code === 0 ? "stopped" : "failed",
        pid: null,
        lspUrl: `http://127.0.0.1:${httpPort}`,
        error: `Engine exited with code=${code} signal=${signal}`,
      };
      engineProcess = null;
    });

    const ready = await waitForPort("127.0.0.1", Number(httpPort), 50, 300);

    if (!ready) {
      engineState.status = "failed";
      engineState.error = "Engine port did not become ready in time";
      return res.status(500).json({
        ok: false,
        status: engineState.status,
        error: engineState.error,
      });
    }

    engineState.status = "ready";

    return res.json({
      ok: true,
      status: engineState.status,
      pid: engineState.pid,
      lspUrl: engineState.lspUrl,
      httpPort,
    });
  } catch (error) {
    engineState.status = "failed";
    engineState.error =
      error instanceof Error ? error.message : String(error);

    return res.status(500).json({
      ok: false,
      status: engineState.status,
      error: engineState.error,
    });
  }
});

const PORT = process.env.LAUNCHER_PORT || 8009;
app.listen(PORT, () => {
  console.log(`Launcher listening on http://127.0.0.1:${PORT}`);
});