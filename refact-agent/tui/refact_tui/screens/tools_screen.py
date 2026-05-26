"""Tool management screen — view and toggle tool groups."""
from __future__ import annotations

from typing import Optional, List, Dict, Any

from textual.app import ComposeResult
from textual.screen import ModalScreen
from textual.binding import Binding
from textual.widgets import Static, Switch, Label
from textual.containers import Vertical, Horizontal, VerticalScroll
from textual.message import Message as TextualMessage
from rich.text import Text

from refact_tui.services.agent_client import AgentClient, ToolGroup


class ToolGroupRow(Static):
    """One tool group with toggle switch."""

    DEFAULT_CSS = """
    ToolGroupRow {
        height: auto;
        padding: 0 1;
        margin: 0 0 1 0;
        border: solid $primary-lighten-3;
    }
    ToolGroupRow .tg-header {
        height: auto;
        margin: 0;
    }
    ToolGroupRow .tg-tools {
        height: auto;
        margin: 0 0 0 2;
        color: $text-muted;
    }
    ToolGroupRow Horizontal {
        height: auto;
    }
    ToolGroupRow Switch {
        margin: 0 1 0 0;
    }
    """

    def __init__(self, group: ToolGroup, index: int, **kwargs):
        super().__init__(**kwargs)
        self.group = group
        self.index = index

    def compose(self) -> ComposeResult:
        all_enabled = all(t.enabled for t in self.group.tools) if self.group.tools else True
        with Horizontal():
            yield Switch(value=all_enabled, id=f"tg-switch-{self.index}")
            category_icon = {"builtin": "🔧", "integration": "🔌", "mcp": "🔗"}.get(
                self.group.category, "📦"
            )
            yield Static(
                Text.from_markup(
                    f"{category_icon} [bold]{self.group.name}[/]  [dim]{self.group.category}[/]"
                ),
                classes="tg-header",
            )
        if self.group.description:
            yield Static(
                Text.from_markup(f"  [dim]{self.group.description[:80]}[/]"),
                classes="tg-tools",
            )
        tool_names = [t.spec.name for t in self.group.tools[:8]]
        if tool_names:
            tools_str = ", ".join(tool_names)
            if len(self.group.tools) > 8:
                tools_str += f" (+{len(self.group.tools) - 8} more)"
            yield Static(
                Text.from_markup(f"  [dim italic]Tools: {tools_str}[/]"),
                classes="tg-tools",
            )


class ToolsScreen(ModalScreen[None]):
    """Modal screen for viewing and toggling tool groups."""

    CSS = """
    ToolsScreen {
        align: center middle;
    }
    #tools-panel {
        width: 75;
        height: auto;
        max-height: 35;
        background: $surface;
        border: solid $primary;
        padding: 1 2;
    }
    #tools-scroll {
        height: auto;
        max-height: 28;
    }
    #tools-footer {
        height: auto;
        margin: 1 0 0 0;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss_screen", "Close", priority=True),
        Binding("q", "dismiss_screen", "Close"),
    ]

    def __init__(self, client: AgentClient, **kwargs):
        super().__init__(**kwargs)
        self._client = client
        self._groups: List[ToolGroup] = []

    def compose(self) -> ComposeResult:
        with Vertical(id="tools-panel"):
            yield Static(
                Text.from_markup(
                    "[bold]🔧 Tool Management[/]  [dim](toggle switches to enable/disable groups)[/]"
                ),
            )
            yield Static("")
            yield VerticalScroll(
                Static("[dim]Loading tool groups…[/]", id="tools-loading"),
                id="tools-scroll",
            )
            yield Static("[dim]Press Esc to close[/]", id="tools-footer")

    async def on_mount(self):
        self._load_tools()

    async def _load_tools(self):
        try:
            self._groups = await self._client.fetch_tool_groups()
        except Exception as e:
            try:
                loading = self.query_one("#tools-loading", Static)
                loading.update(f"[red]Failed to load tools: {e}[/]")
            except Exception:
                pass
            return

        scroll = self.query_one("#tools-scroll", VerticalScroll)
        await scroll.remove_children()

        if not self._groups:
            await scroll.mount(Static("[dim]No tool groups found[/]"))
            return

        for i, group in enumerate(self._groups):
            await scroll.mount(ToolGroupRow(group, i))

    async def on_switch_changed(self, event: Switch.Changed) -> None:
        switch_id = event.switch.id or ""
        if not switch_id.startswith("tg-switch-"):
            return
        idx = int(switch_id.split("-")[-1])
        if idx >= len(self._groups):
            return

        group = self._groups[idx]
        enabled = event.value

        # Build update payload
        updates = []
        for tool in group.tools:
            if tool.spec.source:
                updates.append({
                    "name": group.name,
                    "source": tool.spec.source.model_dump(),
                    "enabled": enabled,
                })
                break  # One update per group is sufficient

        if updates:
            ok = await self._client.update_tool_groups(updates)
            if ok:
                self.notify(f"{'Enabled' if enabled else 'Disabled'} {group.name}")
            else:
                self.notify(f"Failed to update {group.name}", severity="error")

    def action_dismiss_screen(self):
        self.dismiss(None)
