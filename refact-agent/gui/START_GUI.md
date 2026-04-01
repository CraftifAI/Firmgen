# Starting the Refact IDE GUI

This is the React-based GUI for the Refact IDE. It provides a full-featured chat interface and AI toolbox.

## Prerequisites

1. **Refact Agent (LSP Server) must be running**
   - The GUI connects to the refact agent (Rust LSP server)
   - The agent typically runs on a random port (8100-9100) or port 8001
   - Start the agent with: `refact <workspace_path>`

2. **Node.js and npm installed**
   - Dependencies should already be installed in `node_modules/`

## Quick Start

### Option 1: Run with default port (8001)

```bash
cd refact-agent/gui
npm run dev
```

Then open: http://localhost:5173

### Option 2: Run with custom agent port

If your refact agent is running on a different port (e.g., 8187):

```bash
cd refact-agent/gui
REFACT_LSP_URL="http://127.0.0.1:8187" npm run dev
```

Then open: http://localhost:5173

## How It Works

1. **Vite Dev Server**: Runs on port 5173 (default)
2. **Proxy Configuration**: The GUI proxies `/v1/*` requests to the refact agent
3. **Connection**: The GUI connects directly to the refact agent's HTTP API

## Configuration

The GUI uses the following environment variables:

- `REFACT_LSP_URL`: URL of the refact agent (default: `http://127.0.0.1:8001`)
- `REFACT_LSP_PORT`: Port number (alternative to URL)

## Features

- **AI Toolbox**: Left sidebar with AI features (Chat, Fix Bug, Explain Code, etc.)
- **Chat Interface**: Right panel for AI chat with model selection
- **Code Editor Integration**: Can work with IDE integrations (VSCode, JetBrains)
- **Standalone Mode**: Can run as a standalone web app

## Troubleshooting

### GUI can't connect to agent

1. Make sure the refact agent is running
2. Check which port the agent is using: `ps aux | grep refact-lsp | grep --http-port`
3. Set `REFACT_LSP_URL` to match the agent's port

### Port already in use

If port 5173 is already in use, Vite will automatically try the next available port.

### Build for production

```bash
npm run build
```

The built files will be in `dist/chat/` directory.








