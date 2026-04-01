"""Settings screen for model/provider configuration."""
from __future__ import annotations

from typing import Optional, List, Dict, Any

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, ListView, ListItem, Label
from textual.containers import Vertical, VerticalScroll
from rich.text import Text

from refact_tui.services.agent_client import AgentClient, Caps


class ModelListItem(ListItem):
    def __init__(self, model_name: str, n_ctx: int, is_current: bool = False, **kwargs):
        super().__init__(**kwargs)
        self.model_name = model_name
        self.n_ctx = n_ctx
        self.is_current = is_current

    def compose(self) -> ComposeResult:
        marker = "[green bold]> [/]" if self.is_current else "  "
        yield Label(Text.from_markup(f"{marker}[bold]{self.model_name}[/] [dim](ctx: {self.n_ctx})[/]"))


class SettingsScreen(ModalScreen):
    """Modal screen for changing model and viewing providers."""

    CSS = """
    SettingsScreen {
        align: center middle;
    }
    #settings-panel {
        width: 60;
        height: auto;
        max-height: 30;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #model-list {
        height: auto;
        max-height: 20;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss_screen", "Close", priority=True),
        Binding("q", "dismiss_screen", "Close"),
    ]

    def __init__(self, caps: Optional[Caps] = None, current_model: str = "", **kwargs):
        super().__init__(**kwargs)
        self.caps = caps
        self.current_model = current_model
        self.selected_model: Optional[str] = None

    def compose(self) -> ComposeResult:
        with Vertical(id="settings-panel"):
            yield Static(Text.from_markup("[bold]Model Selection[/]  [dim](Enter to select, Esc to cancel)[/]"))
            yield Static("")
            if self.caps:
                items = []
                for name, info in sorted(self.caps.chat_models.items()):
                    items.append(ModelListItem(
                        name,
                        info.n_ctx,
                        is_current=(name == self.current_model),
                    ))
                yield ListView(*items, id="model-list")
            else:
                yield Static("[red]No capabilities loaded - agent may not be connected[/]")

    def on_list_view_selected(self, event: ListView.Selected):
        item = event.item
        if isinstance(item, ModelListItem):
            self.selected_model = item.model_name
            self.dismiss(self.selected_model)

    def action_dismiss_screen(self):
        self.dismiss(None)
