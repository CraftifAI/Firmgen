"""Checkpoint preview and rollback screen."""
from __future__ import annotations

from typing import Optional, List, Dict, Any

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, Button
from textual.containers import Vertical, Horizontal, VerticalScroll
from rich.text import Text
from rich.syntax import Syntax

from refact_tui.services.agent_client import AgentClient


class CheckpointEntry(Static):
    """Single checkpoint entry with restore button."""

    DEFAULT_CSS = """
    CheckpointEntry {
        height: auto;
        padding: 1;
        margin: 0 0 1 0;
        border: solid $primary-lighten-3;
    }
    CheckpointEntry .cp-header {
        height: auto;
    }
    CheckpointEntry .cp-files {
        height: auto;
        margin: 0 0 0 2;
        color: $text-muted;
    }
    CheckpointEntry Horizontal {
        height: auto;
    }
    CheckpointEntry Button {
        margin: 0 1 0 0;
        min-width: 12;
        height: 3;
    }
    """

    def __init__(self, checkpoint: Dict[str, Any], index: int, **kwargs):
        super().__init__(**kwargs)
        self.checkpoint = checkpoint
        self.index = index

    def compose(self) -> ComposeResult:
        cp_id = self.checkpoint.get("checkpoint_id", self.checkpoint.get("id", "?"))
        msg_id = self.checkpoint.get("message_id", "")
        created = self.checkpoint.get("created_at", self.checkpoint.get("created", ""))

        yield Static(
            Text.from_markup(
                f"[bold]📌 Checkpoint[/] [dim]{str(cp_id)[:12]}[/]"
                + (f"  [dim italic]{created}[/]" if created else "")
            ),
            classes="cp-header",
        )

        # Show changed files if available
        files = self.checkpoint.get("files", self.checkpoint.get("changed_files", []))
        if isinstance(files, list) and files:
            for f in files[:5]:
                fname = f if isinstance(f, str) else f.get("file_name", f.get("path", "?"))
                yield Static(Text.from_markup(f"  [dim]• {fname}[/]"), classes="cp-files")
            if len(files) > 5:
                yield Static(
                    Text.from_markup(f"  [dim]… and {len(files) - 5} more files[/]"),
                    classes="cp-files",
                )

        yield Horizontal(
            Button("↩ Restore", id=f"restore-{self.index}", variant="warning"),
        )


class CheckpointScreen(ModalScreen[Optional[str]]):
    """Modal screen for previewing and restoring checkpoints.

    Returns the checkpoint_id that was restored, or None if dismissed.
    """

    CSS = """
    CheckpointScreen {
        align: center middle;
    }
    #cp-panel {
        width: 70;
        height: auto;
        max-height: 35;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #cp-scroll {
        height: auto;
        max-height: 28;
    }
    #cp-footer {
        height: auto;
        margin: 1 0 0 0;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss_screen", "Close", priority=True),
        Binding("q", "dismiss_screen", "Close"),
    ]

    def __init__(self, client: AgentClient, chat_id: str, **kwargs):
        super().__init__(**kwargs)
        self._client = client
        self._chat_id = chat_id
        self._checkpoints: List[Dict[str, Any]] = []

    def compose(self) -> ComposeResult:
        with Vertical(id="cp-panel"):
            yield Static(
                Text.from_markup(
                    "[bold]📌 Checkpoints[/]  [dim](restore to undo file changes)[/]"
                ),
            )
            yield Static("")
            yield VerticalScroll(
                Static("[dim]Loading checkpoints…[/]", id="cp-loading"),
                id="cp-scroll",
            )
            yield Static("[dim]Press Esc to close[/]", id="cp-footer")

    async def on_mount(self):
        self._load_checkpoints()

    async def _load_checkpoints(self):
        try:
            self._checkpoints = await self._client.fetch_checkpoints_preview(self._chat_id)
        except Exception as e:
            try:
                loading = self.query_one("#cp-loading", Static)
                loading.update(f"[red]Failed to load checkpoints: {e}[/]")
            except Exception:
                pass
            return

        scroll = self.query_one("#cp-scroll", VerticalScroll)
        await scroll.remove_children()

        if not self._checkpoints:
            await scroll.mount(Static("[dim]No checkpoints found for this chat[/]"))
            return

        for i, cp in enumerate(self._checkpoints):
            await scroll.mount(CheckpointEntry(cp, i))

    async def on_button_pressed(self, event: Button.Pressed) -> None:
        btn_id = event.button.id or ""
        if not btn_id.startswith("restore-"):
            return
        event.stop()

        idx = int(btn_id.split("-", 1)[1])
        if idx >= len(self._checkpoints):
            return

        cp = self._checkpoints[idx]
        cp_id = cp.get("checkpoint_id", cp.get("id", ""))
        if not cp_id:
            self.notify("No checkpoint ID found", severity="error")
            return

        event.button.disabled = True
        event.button.label = "Restoring…"

        ok = await self._client.restore_checkpoint(self._chat_id, str(cp_id))
        if ok:
            self.notify(f"Checkpoint restored: {str(cp_id)[:12]}")
            self.dismiss(str(cp_id))
        else:
            self.notify("Failed to restore checkpoint", severity="error")
            event.button.disabled = False
            event.button.label = "↩ Restore"

    def action_dismiss_screen(self):
        self.dismiss(None)
