# CraftifAI Agent TUI

A full-featured terminal UI dashboard for the CraftifAI / Refact Agent, built with [Textual](https://textual.textualize.io/).

## Features

- **Streaming Chat** — Real-time SSE streaming with live token display
- **Rich Rendering** — Markdown, syntax-highlighted code blocks, colorized diffs
- **Thinking Blocks** — Collapsible reasoning model thinking display (Claude, etc.)
- **Tool Calls** — Inline display with collapsible results
- **Tool Confirmation** — Approval modal for dangerous tool operations (file writes, commands)
- **Tool Management** — View and toggle tool groups on/off (`Ctrl+T`)
- **Checkpoints** — Preview and restore file changes (`Ctrl+Z`)
- **Chat History** — Persistent conversation history across sessions (`Ctrl+H`)
- **File Browser** — Project file tree in the sidebar with file type icons
- **Tool Status** — Recent tool call history with success/failure indicators
- **Workflow Panel** — SSE workflow events, task progress, pause/resume
- **Model Switching** — Interactive model selection via `Ctrl+M` or `/model`
- **Chat Modes** — Switch between AGENT, EXPLORE, NOTOOLS, CONFIGURE
- **ESP32 Integration** — Auto-detects ESP32 projects and passes context
- **Status Bar** — Model info, token usage, chat mode, AST/VecDB indexing progress

## Installation

```bash
cd refact-agent/tui
pip install -e .
```

## Usage

```bash
# Auto-detect agent (tries CraftifAI desktop on port 8486, then standalone on 8001)
refact-tui /path/to/project

# Connect to the CraftifAI desktop app
refact-tui --port 8486 /path/to/project

# Connect to a standalone agent
refact-tui --port 8001 /path/to/project

# Start a new agent with a specific binary
refact-tui --lsp-binary ./refact-lsp /path/to/project

# Specify model and chat mode
refact-tui --model gpt-4 --chat-mode AGENT --port 8486 .

# Specify ESP32 projects directory
refact-tui --esp32-projects-path ~/esp32-projects --port 8486 .
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in input |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+M` | Switch model |
| `Ctrl+T` | Tool management |
| `Ctrl+H` | Chat history |
| `Ctrl+Z` | Checkpoints (rollback) |
| `Ctrl+N` | New chat session |
| `Ctrl+L` | Clear chat |
| `Ctrl+P` | Pause/Resume workflow |
| `Escape` | Stop streaming |
| `Ctrl+Q` | Quit |
| `F1` | Help screen |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/model [name]` | Switch model or open model picker |
| `/mode [MODE]` | Switch chat mode (AGENT, EXPLORE, NOTOOLS, CONFIGURE) |
| `/tools` | View and toggle tool groups |
| `/checkpoints` | Preview and restore file checkpoints |
| `/history` | Browse and restore saved conversations |
| `/clear` | Clear chat history |
| `/new` | Start new chat session |
| `/export` | Export chat to JSON file |
| `/pause` | Pause/resume workflow |
| `/help` | Show help screen |
| `/exit` | Exit |

## Architecture

The TUI connects to the Refact Agent engine via HTTP (same API the web GUI uses):

```
refact-tui  ──HTTP/SSE──>  refact-lsp (port 8486 or 8001)
```

Key endpoints used:
- `POST /v1/chat` (stream=true) — chat with SSE streaming
- `GET /v1/caps` — model list and capabilities
- `GET /v1/tools` — tool groups with enable/disable state
- `POST /v1/tools` — update tool group configuration
- `POST /v1/tools-check-if-confirmation-needed` — tool call confirmation
- `POST /v1/checkpoints-preview` — list file checkpoints
- `POST /v1/checkpoints-restore` — restore a checkpoint
- `POST /v1/links` — follow-up action buttons
- `GET /v1/workflow/events` — SSE workflow events
- `GET /v1/rag-status` — AST/VecDB indexing status
- `POST /v1/at-command-completion` — @-command completions

## Tool Confirmation

When the agent makes tool calls that could modify files or run commands, the TUI checks
with the backend if confirmation is needed. If so, a modal appears where you can:

- **Approve All** — Let all pending tool calls execute
- **Deny All** — Block all pending tool calls
- **Per-tool** — Approve or deny each tool call individually

Denied tool calls are reported back to the agent as denied, and it can adjust its plan.

## Chat History

Conversations are automatically saved to:
- **Windows**: `%APPDATA%/craftifai/tui_history/`
- **Linux/macOS**: `~/.config/craftifai/tui_history/`

Use `Ctrl+H` or `/history` to browse and restore previous conversations.

## Development

```bash
# Install in development mode
pip install -e .

# Run with Textual dev tools
textual run --dev refact_tui.app:RefactTUI

# Run Textual console for debugging
textual console
```
