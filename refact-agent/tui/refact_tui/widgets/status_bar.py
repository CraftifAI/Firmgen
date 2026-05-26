"""Bottom status bar showing model info, token usage, and indexing status."""
from __future__ import annotations

from typing import Optional

from textual.widget import Widget
from textual.app import ComposeResult
from textual.widgets import Static
from textual.reactive import reactive
from rich.text import Text

from refact_tui.services.agent_client import Usage


class StatusBar(Widget):
    """Application status bar at the bottom of the screen."""

    DEFAULT_CSS = """
    StatusBar {
        dock: bottom;
        height: 1;
        background: $surface;
    }
    """

    model_name: reactive[str] = reactive("", layout=True)
    is_streaming: reactive[bool] = reactive(False)
    ast_status: reactive[str] = reactive("")
    vecdb_status: reactive[str] = reactive("")
    usage_text: reactive[str] = reactive("")
    project_path: reactive[str] = reactive("")
    chat_mode: reactive[str] = reactive("AGENT")

    def render(self):
        parts = []

        if self.model_name:
            parts.append(f"[bold cyan]{self.model_name}[/]")

        if self.is_streaming:
            parts.append("[yellow bold]⏳ streaming…[/]")
        else:
            parts.append("[green]● ready[/]")

        if self.chat_mode:
            parts.append(f"[dim]{self.chat_mode}[/]")

        if self.usage_text:
            parts.append(f"[dim]{self.usage_text}[/]")

        if self.ast_status:
            parts.append(self.ast_status)

        if self.vecdb_status:
            parts.append(self.vecdb_status)

        if self.project_path:
            path_short = self.project_path
            if len(path_short) > 30:
                path_short = "…" + path_short[-27:]
            parts.append(f"[dim]{path_short}[/]")

        return Text.from_markup(" │ ".join(parts))

    def update_usage(self, usage: Optional[Usage]):
        if usage:
            total = usage.prompt_tokens + usage.completion_tokens
            cache_info = ""
            if usage.cache_read_input_tokens > 0:
                cache_info = f" cache:{usage.cache_read_input_tokens}"
            self.usage_text = f"tokens: {total:,}{cache_info}"
        else:
            self.usage_text = ""

    def update_rag_status(self, status: dict):
        if ast := status.get("ast"):
            state = ast.get("state", "")
            if state == "indexing":
                parsed = ast.get("files_total", 0) - ast.get("files_unparsed", 0)
                self.ast_status = f"[yellow]AST {parsed}/{ast.get('files_total', 0)}[/]"
            elif state == "done":
                total_f = ast.get("ast_index_files_total", 0)
                total_s = ast.get("ast_index_symbols_total", 0)
                self.ast_status = f"[green]AST {total_f}f/{total_s}s[/]"
            else:
                self.ast_status = f"[dim]AST {state}[/]"
        else:
            self.ast_status = ""

        if vecdb := status.get("vecdb"):
            state = vecdb.get("state", "")
            if state not in ("done", "idle"):
                parsed = vecdb.get("files_total", 0) - vecdb.get("files_unprocessed", 0)
                self.vecdb_status = f"[yellow]VecDB {parsed}/{vecdb.get('files_total', 0)}[/]"
            else:
                self.vecdb_status = f"[green]VecDB {vecdb.get('db_size', 0)} records[/]"
        else:
            self.vecdb_status = ""
