# Refact Agent TUI

A full-featured terminal UI dashboard for the Refact Agent, built with [Textual](https://textual.textualize.io/).

## Features

- **Streaming Chat** — Real-time SSE streaming with live token display
- **Rich Rendering** — Markdown, syntax-highlighted code blocks, colorized diffs
- **Tool Calls** — Inline display with collapsible results
- **File Browser** — Project file tree in the sidebar
- **Tool Status** — Recent tool call history with success/failure indicators
- **Workflow Panel** — SSE workflow events, task progress, pause/resume
- **Model Switching** — Interactive model selection via `Ctrl+M` or `/model`
- **Status Bar** — Model info, token usage, AST/VecDB indexing progress

## Installation

```bash
cd refact-agent/tui
pip install -e .
```

## Usage

```bash
# Auto-detect refact-lsp binary or connect to default port
refact-tui /path/to/project

# Connect to an already-running agent
refact-tui --port 8001 /path/to/project

# Start a new agent with a specific binary
refact-tui --lsp-binary ./refact-lsp /path/to/project

# Specify model
refact-tui --model gpt-4 --port 8001 .
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in input |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+M` | Switch model |
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
| `/clear` | Clear chat history |
| `/new` | Start new chat session |
| `/export` | Export chat to JSON file |
| `/pause` | Pause/resume workflow |
| `/help` | Show help screen |
| `/exit` | Exit |

## Architecture

The TUI connects to the Refact Agent engine via HTTP (same API the web GUI uses):

```
refact-tui  ──HTTP/SSE──>  refact-lsp (port 8001)
```

Key endpoints used:
- `POST /v1/chat` (stream=true) — chat with SSE streaming
- `GET /v1/caps` — model list and capabilities
- `GET /v1/workflow/events` — SSE workflow events
- `GET /v1/rag-status` — AST/VecDB indexing status
- `POST /v1/at-command-completion` — @-command completions

## Development

```bash
# Install in development mode
pip install -e .

# Run with Textual dev tools
textual run --dev refact_tui.app:RefactTUI

# Run Textual console for debugging
textual console
```
