"""Tool confirmation modal screen.

Displays pending tool calls that require user approval before execution.
Matches the GUI's tool-confirmation behavior from /v1/tools-check-if-confirmation-needed.
"""
from __future__ import annotations

from typing import Optional, List, Dict

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, Button, Label
from textual.containers import Vertical, Horizontal, VerticalScroll
from rich.text import Text

from refact_tui.services.agent_client import ConfirmationPauseReason


class ConfirmationItem(Static):
    """Single tool call requiring confirmation."""

    def __init__(self, reason: ConfirmationPauseReason, index: int, **kwargs):
        super().__init__(**kwargs)
        self.reason = reason
        self.index = index
        self.decision: Optional[str] = None  # "approve" or "deny"

    def compose(self) -> ComposeResult:
        icon = "🔒" if self.reason.type == "confirmation" else "🚫"
        yield Static(
            Text.from_markup(
                f"\n{icon} [bold]{self.reason.command or 'tool call'}[/]\n"
                f"   [dim]Rule: {self.reason.rule or 'requires approval'}[/]\n"
                f"   [dim]Tool call ID: {self.reason.tool_call_id[:16]}…[/]"
            ),
            classes="confirm-item-header",
        )
        yield Horizontal(
            Button("✓ Approve", id=f"approve-{self.index}", variant="success", classes="confirm-btn"),
            Button("✗ Deny", id=f"deny-{self.index}", variant="error", classes="confirm-btn"),
            classes="confirm-item-buttons",
        )


class ConfirmScreen(ModalScreen[Dict[str, str]]):
    """Modal screen for approving/denying tool calls.

    Returns a dict mapping tool_call_id → "approve" | "deny".
    """

    CSS = """
    ConfirmScreen {
        align: center middle;
    }
    #confirm-panel {
        width: 70;
        height: auto;
        max-height: 35;
        background: $surface;
        border: heavy $warning;
        padding: 1 2;
    }
    #confirm-title {
        text-align: center;
        margin: 0 0 1 0;
    }
    #confirm-scroll {
        height: auto;
        max-height: 25;
    }
    .confirm-item-header {
        height: auto;
        margin: 0 0 0 1;
    }
    .confirm-item-buttons {
        height: auto;
        margin: 0 0 1 1;
    }
    .confirm-btn {
        margin: 0 1 0 0;
        min-width: 14;
        height: 3;
    }
    #confirm-actions {
        height: auto;
        margin: 1 0 0 0;
        align: center middle;
    }
    .confirm-all-btn {
        margin: 0 1;
        min-width: 18;
        height: 3;
    }
    """

    BINDINGS = [
        Binding("escape", "deny_all", "Deny All", priority=True),
    ]

    def __init__(self, reasons: List[ConfirmationPauseReason], **kwargs):
        super().__init__(**kwargs)
        self.reasons = reasons
        self.decisions: Dict[str, str] = {}
        self._items: List[ConfirmationItem] = []

    def compose(self) -> ComposeResult:
        with Vertical(id="confirm-panel"):
            yield Static(
                Text.from_markup(
                    "[bold yellow]⚠ Tool Confirmation Required[/]\n"
                    "[dim]The following tool calls need your approval before executing.[/]"
                ),
                id="confirm-title",
            )
            with VerticalScroll(id="confirm-scroll"):
                for i, reason in enumerate(self.reasons):
                    item = ConfirmationItem(reason, i)
                    self._items.append(item)
                    yield item
            with Horizontal(id="confirm-actions"):
                yield Button("✓ Approve All", id="approve-all", variant="success", classes="confirm-all-btn")
                yield Button("✗ Deny All", id="deny-all", variant="error", classes="confirm-all-btn")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        btn_id = event.button.id or ""
        event.stop()

        if btn_id == "approve-all":
            for reason in self.reasons:
                self.decisions[reason.tool_call_id] = "approve"
            self.dismiss(self.decisions)
            return

        if btn_id == "deny-all":
            self.action_deny_all()
            return

        if btn_id.startswith("approve-"):
            idx = int(btn_id.split("-", 1)[1])
            reason = self.reasons[idx]
            self.decisions[reason.tool_call_id] = "approve"
            # Disable the buttons for this item
            event.button.disabled = True
            try:
                deny_btn = self.query_one(f"#deny-{idx}", Button)
                deny_btn.disabled = True
            except Exception:
                pass
            event.button.label = "✓ Approved"
            # Check if all decisions are made
            if len(self.decisions) >= len(self.reasons):
                self.dismiss(self.decisions)
            return

        if btn_id.startswith("deny-"):
            idx = int(btn_id.split("-", 1)[1])
            reason = self.reasons[idx]
            self.decisions[reason.tool_call_id] = "deny"
            event.button.disabled = True
            try:
                approve_btn = self.query_one(f"#approve-{idx}", Button)
                approve_btn.disabled = True
            except Exception:
                pass
            event.button.label = "✗ Denied"
            if len(self.decisions) >= len(self.reasons):
                self.dismiss(self.decisions)
            return

    def action_deny_all(self):
        for reason in self.reasons:
            if reason.tool_call_id not in self.decisions:
                self.decisions[reason.tool_call_id] = "deny"
        self.dismiss(self.decisions)
