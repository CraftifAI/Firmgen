"""Chat history browser screen."""
from __future__ import annotations

from typing import Optional, List

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, ListView, ListItem, Label, Button
from textual.containers import Vertical, Horizontal, VerticalScroll
from rich.text import Text

from refact_tui.services.history import ChatHistory, ChatHistoryEntry


class HistoryListItem(ListItem):
    """A single chat history entry in the list."""

    def __init__(self, entry: ChatHistoryEntry, **kwargs):
        super().__init__(**kwargs)
        self.entry = entry

    def compose(self) -> ComposeResult:
        yield Label(Text.from_markup(
            f"[bold]{self.entry.title}[/]\n"
            f"  [dim]{self.entry.model} · {self.entry.message_count} messages[/]"
        ))


class HistoryScreen(ModalScreen[Optional[str]]):
    """Modal screen for browsing and restoring chat history.

    Returns the chat_id to restore, or None if dismissed.
    """

    CSS = """
    HistoryScreen {
        align: center middle;
    }
    #history-panel {
        width: 70;
        height: auto;
        max-height: 35;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #history-list {
        height: auto;
        max-height: 25;
    }
    #history-footer {
        height: auto;
        margin: 1 0 0 0;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss_screen", "Close", priority=True),
        Binding("q", "dismiss_screen", "Close"),
        Binding("delete", "delete_selected", "Delete", priority=True),
    ]

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._history = ChatHistory()
        self._entries: List[ChatHistoryEntry] = []

    def compose(self) -> ComposeResult:
        with Vertical(id="history-panel"):
            yield Static(
                Text.from_markup(
                    "[bold]📋 Chat History[/]  [dim](Enter to restore, Delete to remove)[/]"
                ),
            )
            yield Static("")
            self._entries = self._history.list_chats()
            if self._entries:
                items = [HistoryListItem(entry) for entry in self._entries]
                yield ListView(*items, id="history-list")
            else:
                yield Static("[dim]No saved conversations found[/]")
            yield Static("[dim]Press Esc to close[/]", id="history-footer")

    def on_list_view_selected(self, event: ListView.Selected) -> None:
        item = event.item
        if isinstance(item, HistoryListItem):
            self.dismiss(item.entry.chat_id)

    def action_delete_selected(self) -> None:
        try:
            list_view = self.query_one("#history-list", ListView)
            if list_view.highlighted_child and isinstance(list_view.highlighted_child, HistoryListItem):
                entry = list_view.highlighted_child.entry
                self._history.delete(entry.chat_id)
                self.notify(f"Deleted: {entry.title[:40]}")
                # Refresh the list
                list_view.highlighted_child.remove()
        except Exception:
            pass

    def action_dismiss_screen(self):
        self.dismiss(None)
