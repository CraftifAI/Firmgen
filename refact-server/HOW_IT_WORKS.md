# How the Refact Agent and Web UI Work Together

## Architecture Overview

When you run `refact <workspace_path>`, here's what happens:

### 1. The `refact` Command (Python CLI)

The `refact` command is a Python wrapper located at:
```
refact-agent/engine/python_binding_and_cmdline/refact/cli_main.py
```

**What it does:**
- Starts an interactive chat interface in your terminal
- Launches a subprocess running `refact-lsp` (the Rust binary)
- The `refact-lsp` binary runs an HTTP server on a **random port** (8100-9100)
- The Python CLI reads stderr from `refact-lsp` to find the port
- The Python CLI then uses that port to communicate with the agent

### 2. The `refact-lsp` Binary (Rust Agent)

The actual agent is a Rust binary that:
- Runs as a subprocess started by the Python CLI
- Starts an HTTP server on a random port (e.g., 8187)
- Provides these endpoints:
  - `/v1/ping` - Health check
  - `/v1/caps` - Get capabilities (models, etc.)
  - `/v1/tools` - Get available tools (including C2000 tools!)
  - `/v1/chat` - Chat endpoint
  - And many more...

### 3. The Web UI (Python FastAPI)

The Web UI (`refact-server`) is a separate Python application that:
- Runs on port **8008** (default)
- Provides a web interface with tabs
- The **Chat tab** connects to the `refact-lsp` HTTP server
- The **C2000 Tools tab** provides direct C2000 management

## How Port Detection Works

The Web UI automatically detects which port the agent is using:

1. **First**: Checks the process list (`ps aux`) to find `refact-lsp` and extract the port from `--http-port=XXXX`
2. **Then**: Verifies the port is responding to `/v1/ping`
3. **Fallback**: If process list check fails, scans ports 8100-9100 sequentially

## Current Setup (Your System)

Based on your `ps aux` output:

```
refact-lsp --http-port=8187
```

So your agent is running on **port 8187**.

## How to Use

### Step 1: Start the Agent

```bash
# In one terminal
refact /path/to/workspace

# This will:
# - Start refact-lsp on a random port (e.g., 8187)
# - Open an interactive chat prompt
# - Keep running until you type 'exit' or Ctrl+D
```

**Important**: Keep this terminal open! The agent needs to keep running.

### Step 2: Start the Web UI

```bash
# In another terminal
cd refact-server
python -m refact_webgui.webgui.webgui
```

This starts the Web UI on port **8008**.

### Step 3: Use the Chat Tab

1. Open http://127.0.0.1:8008 in your browser
2. Click the **"Chat"** tab
3. Click **"Check Connection"** button
4. The Web UI will automatically detect port 8187 (or whatever port your agent is using)
5. You'll see: "Connected (port 8187, auto-detected)"
6. Start chatting! The agent has access to all C2000 tools

## Available Endpoints

### Agent Endpoints (port 8187 in your case)

- `http://127.0.0.1:8187/v1/ping` - Health check
- `http://127.0.0.1:8187/v1/caps` - Get capabilities
- `http://127.0.0.1:8187/v1/tools` - Get available tools
- `http://127.0.0.1:8187/v1/chat` - Chat endpoint

### Web UI Endpoints (port 8008)

- `http://127.0.0.1:8008/` - Main Web UI
- `http://127.0.0.1:8008/list-plugins` - List available tabs
- `http://127.0.0.1:8008/tab-chat-ping` - Check agent connection
- `http://127.0.0.1:8008/tab-chat-send` - Send chat message

## C2000 Tools Available

Your agent has these C2000 tools (visible in `/v1/tools`):

1. **c2000_project_create** - Create CCS projects
2. **c2000_build** - Build projects
3. **c2000_flash** - Flash to device
4. **c2000_uart_capture** - Capture UART output
5. **c2000_target_detect** - Detect hardware
6. **c2000_example_list** - List C2000Ware examples
7. **c2000_config_validate** - Validate configuration
8. **c2000_code_evaluator** - AI code evaluation
9. **c2000_sysconfig_modify** - Modify .syscfg files
10. **c2000_projectspec_modify** - Modify .projectspec files

## Quick Test

Test if everything is working:

```bash
# 1. Check agent is running
curl http://127.0.0.1:8187/v1/ping
# Should return: pong

# 2. Check Web UI is running
curl http://127.0.0.1:8008/ping
# Should return: {"message":"pong"}

# 3. Check agent connection from Web UI
curl http://127.0.0.1:8008/tab-chat-ping
# Should return connection status
```

## Summary

- **Agent** (`refact` command): Runs `refact-lsp` on random port (8100-9100)
- **Web UI**: Runs on port 8008, auto-detects agent port
- **Chat Tab**: Connects Web UI to Agent for chatting
- **C2000 Tools Tab**: Direct C2000 management interface
- **Both use the same agent** - same tools, same capabilities!

The key insight: The `refact` command is an **interactive chat interface** that happens to start an HTTP server in the background. The Web UI connects to that HTTP server to provide a web-based chat interface.









