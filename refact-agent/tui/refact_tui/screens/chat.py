"""Help screen overlay."""
from __future__ import annotations

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, Markdown
from textual.containers import Vertical, VerticalScroll


HELP_TEXT = """\
# CraftifAI Agent TUI

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in input |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+L` | Clear chat |
| `Ctrl+N` | New chat session |
| `Ctrl+M` | Switch model |
| `Ctrl+T` | Tool management |
| `Ctrl+H` | Chat history |
| `Ctrl+Z` | Checkpoints (rollback) |
| `Ctrl+P` | Pause/Resume workflow |
| `Escape` | Stop streaming |
| `Ctrl+Q` | Quit |
| `F1` | This help screen |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/model [name]` | Switch model or list available models |
| `/mode [MODE]` | Switch chat mode (AGENT, EXPLORE, NOTOOLS, CONFIGURE) |
| `/tools` | View and toggle tool groups |
| `/checkpoints` | Preview and restore file checkpoints |
| `/history` | Browse and restore saved conversations |
| `/clear` | Clear chat history |
| `/new` | Start a new chat session |
| `/export` | Export chat to JSON file |
| `/pause` | Pause/resume workflow |
| `/help` | Show this help |
| `/exit` | Exit the application |

## Chat Features

- **Streaming**: Responses stream in real-time with a live text display
- **Tool Calls**: Agent tool calls shown inline with results; **confirmation required** for dangerous operations
- **Thinking Blocks**: Reasoning model thoughts shown in collapsible blocks
- **Markdown**: Assistant responses render as formatted Markdown
- **Code Blocks**: Syntax-highlighted code blocks
- **Diffs**: File changes shown as colorized diffs
- **Context Files**: Attached files shown with file paths and line ranges
- **Chat History**: Conversations auto-saved and restorable across sessions
- **Checkpoints**: Preview and rollback file changes made during chat

## Sidebar Panels

- **Files**: Project file tree (toggle with `Ctrl+B`)
- **Tools**: Recent tool call history with status indicators
- **Workflow**: Workflow events and task progress

## Connecting to Agent

```
refact-tui .                             # auto-detect (tries port 8486 then 8001)
refact-tui --port 8486 .                # connect to CraftifAI desktop app
refact-tui --port 8001 .                # connect to standalone agent
refact-tui --lsp-binary ./refact-lsp .   # start new agent
refact-tui --chat-mode EXPLORE .         # use EXPLORE mode instead of AGENT
refact-tui --esp32-projects-path /path . # specify ESP32 projects directory
```
"""


class HelpScreen(ModalScreen):
    """Modal help screen with keyboard shortcuts and command reference."""

    CSS = """
    HelpScreen {
        align: center middle;
    }
    #help-panel {
        width: 75;
        height: auto;
        max-height: 38;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #help-scroll {
        height: auto;
        max-height: 33;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss_screen", "Close", priority=True),
        Binding("q", "dismiss_screen", "Close"),
        Binding("f1", "dismiss_screen", "Close", priority=True),
    ]

    def compose(self) -> ComposeResult:
        with Vertical(id="help-panel"):
            yield VerticalScroll(
                Markdown(HELP_TEXT),
                id="help-scroll",
            )
            yield Static("[dim]Press Esc or q to close[/]")

    def action_dismiss_screen(self):
        self.dismiss()
