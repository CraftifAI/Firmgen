"""Chat input area with multi-line support, command handling, and @-completions."""
from __future__ import annotations

import asyncio
from typing import Optional, List, Callable, Awaitable

from textual.app import ComposeResult
from textual.binding import Binding
from textual.widget import Widget
from textual.widgets import TextArea, Static, Button
from textual.containers import Horizontal
from textual.message import Message
from textual import events
from rich.text import Text


class ChatTextArea(TextArea):
    """TextArea variant where Enter submits the message instead of inserting a newline.

    Shift+Enter keeps the default behavior and inserts a newline.
    """

    BINDINGS = [
        Binding("enter", "submit", "Send", show=False, priority=True),
        Binding("return", "submit", "Send", show=False, priority=True),
    ]

    def on_key(self, event: events.Key) -> None:
        """Backup: intercept Enter in case binding does not fire (e.g. focus/consumption)."""
        key = event.key.lower()
        if key not in ("enter", "return"):
            return
        if event.shift:
            return

        text = self.text.strip()
        if not text:
            event.prevent_default()
            return

        event.prevent_default()
        event.stop()
        self._do_submit(text)

    def action_submit(self) -> None:
        """Called when Enter/Return binding fires. Submit current text and clear."""
        text = self.text.strip()
        if not text:
            return
        self._do_submit(text)

    def _do_submit(self, text: str) -> None:
        chat_input = None
        for node in self.ancestors:
            if isinstance(node, ChatInput):
                chat_input = node
                break
        if chat_input is not None:
            chat_input.post_message(ChatInput.Submitted(text))
        self.text = ""


class ChatInput(Widget):
    """Multi-line chat input with Enter to send."""

    DEFAULT_CSS = """
    ChatInput {
        height: auto;
        max-height: 4;
        dock: bottom;
    }
    """

    class Submitted(Message):
        """Fired when user submits input."""
        def __init__(self, text: str):
            super().__init__()
            self.text = text

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._completion_fn: Optional[Callable] = None

    def compose(self) -> ComposeResult:
        yield Horizontal(
            Static(Text.from_markup("[bold cyan]>[/] "), classes="input-prompt"),
            ChatTextArea(id="chat-textarea", language=None),
            Button("Send", id="send-btn", classes="send-button", variant="primary"),
            classes="input-row",
        )

    def on_mount(self):
        ta = self.query_one("#chat-textarea", ChatTextArea)
        ta.show_line_numbers = False
        ta.focus()

    def on_button_pressed(self, event: Button.Pressed) -> None:
        """Send button clicked — same logic as Enter."""
        if event.button.id != "send-btn":
            return
        ta = self.query_one("#chat-textarea", ChatTextArea)
        text = ta.text.strip()
        if not text:
            return
        ta.text = ""
        self.post_message(ChatInput.Submitted(text))
        ta.focus()

    def set_completion_fn(self, fn: Optional[Callable]):
        """Set the async function to call for @-command completions.
        fn(query, cursor_pos) -> dict with 'completions' and 'replace' keys.
        """
        self._completion_fn = fn

    def set_disabled(self, disabled: bool):
        ta = self.query_one("#chat-textarea", ChatTextArea)
        ta.read_only = disabled
        try:
            btn = self.query_one("#send-btn", Button)
            btn.disabled = disabled
        except Exception:
            pass

    def focus_input(self):
        ta = self.query_one("#chat-textarea", ChatTextArea)
        ta.focus()
