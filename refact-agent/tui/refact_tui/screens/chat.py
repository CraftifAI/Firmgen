"""Help screen overlay."""
from __future__ import annotations

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, Markdown
from textual.containers import Vertical, VerticalScroll


HELP_TEXT = """\
# Refact Agent TUI

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in input |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+L` | Clear chat |
| `Ctrl+N` | New chat session |
| `Ctrl+M` | Switch model |
| `Ctrl+P` | Pause/Resume workflow |
| `Escape` | Stop streaming |
| `Ctrl+Q` | Quit |
| `F1` | This help screen |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/model [name]` | Switch model or list available models |
| `/clear` | Clear chat history |
| `/new` | Start a new chat session |
| `/export` | Export chat to file |
| `/help` | Show this help |
| `/exit` | Exit the application |

## Chat Features

- **Streaming**: Responses stream in real-time with a live text display
- **Tool Calls**: Agent tool calls are shown inline; results are collapsible
- **Markdown**: Assistant responses render as formatted Markdown
- **Code Blocks**: Syntax-highlighted code blocks
- **Diffs**: File changes shown as colorized diffs
- **Context Files**: Attached files shown with file paths and line ranges

## Sidebar Panels

- **Files**: Project file tree (toggle with `Ctrl+B`)
- **Tools**: Recent tool call history with status indicators
- **Workflow**: Workflow events and task progress

## Connecting to Agent

```
refact-tui .                          # auto-detect agent
refact-tui --port 8001 .             # connect to running agent
refact-tui --lsp-binary ./refact-lsp . # start new agent
```
"""


class HelpScreen(ModalScreen):
    """Modal help screen with keyboard shortcuts and command reference."""

    CSS = """
    HelpScreen {
        align: center middle;
    }
    #help-panel {
        width: 70;
        height: auto;
        max-height: 35;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #help-scroll {
        height: auto;
        max-height: 30;
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
