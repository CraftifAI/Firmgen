"""Collapsible sidebar with file tree, tool status, and workflow panels."""
from __future__ import annotations

import json
import os
from typing import Optional, List, Dict, Any

from textual.app import ComposeResult
from textual.widget import Widget
from textual.widgets import Static, Tree, Collapsible, ListView, ListItem, Label
from textual.containers import Vertical, VerticalScroll
from textual.reactive import reactive
from rich.text import Text


SKIP_DIRS = {
    "node_modules", "__pycache__", ".git", ".hg", ".svn", "target",
    "build", "dist", ".tox", ".venv", "venv", ".cache", ".mypy_cache",
    ".pytest_cache", ".eggs", "*.egg-info", ".idea", ".vscode",
}


class FileTreePanel(Widget):
    """Project file tree browser with lazy-loading directories."""

    DEFAULT_CSS = """
    FileTreePanel { height: 1fr; }
    """

    def __init__(self, root_path: str = ".", **kwargs):
        super().__init__(**kwargs)
        self.root_path = os.path.abspath(root_path)

    def compose(self) -> ComposeResult:
        tree: Tree[str] = Tree(os.path.basename(self.root_path) or "/", id="file-tree")
        tree.root.data = self.root_path
        self._populate(tree.root, self.root_path, depth=0)
        tree.root.expand()
        yield tree

    def _populate(self, node, path: str, depth: int):
        if depth > 3:
            return
        try:
            entries = sorted(os.listdir(path))
        except (PermissionError, OSError):
            return
        dirs = [
            e for e in entries
            if os.path.isdir(os.path.join(path, e))
            and not e.startswith(".")
            and e not in SKIP_DIRS
        ]
        files = [
            e for e in entries
            if os.path.isfile(os.path.join(path, e))
            and not e.startswith(".")
        ]

        for d in dirs[:40]:
            full = os.path.join(path, d)
            child = node.add(f"[bold]{d}/[/]", data=full)
            if depth < 1:
                self._populate(child, full, depth + 1)

        for f in files[:40]:
            node.add_leaf(f, data=os.path.join(path, f))

        remaining = len(dirs) - 40 + max(0, len(files) - 40)
        if remaining > 0:
            node.add_leaf(f"[dim]... {remaining} more entries[/]")


class ToolStatusPanel(Widget):
    """Shows recent tool calls and their status."""

    DEFAULT_CSS = """
    ToolStatusPanel { height: auto; max-height: 20; }
    """

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._entries: List[Dict[str, str]] = []

    def compose(self) -> ComposeResult:
        yield VerticalScroll(Static("[dim]No tool calls yet[/]", id="tool-entries"), id="tool-scroll")

    def add_tool_call(self, name: str, status: str = "running"):
        self._entries.append({"name": name, "status": status})
        if len(self._entries) > 20:
            self._entries = self._entries[-20:]
        self._refresh_display()

    def update_last(self, status: str):
        if self._entries:
            self._entries[-1]["status"] = status
            self._refresh_display()

    def _refresh_display(self):
        icons = {"running": "[yellow]⏳[/]", "ok": "[green]✓[/]", "failed": "[red]✗[/]"}
        lines = []
        for e in self._entries:
            icon = icons.get(e["status"], "?")
            lines.append(f"{icon} {e['name']}")
        try:
            entry_widget = self.query_one("#tool-entries", Static)
            entry_widget.update("\n".join(lines) if lines else "[dim]No tool calls yet[/]")
        except Exception:
            pass


class WorkflowPanel(Widget):
    """Shows workflow events, task list, and pause/resume controls."""

    DEFAULT_CSS = """
    WorkflowPanel {
        height: auto;
        max-height: 20;
    }
    .wf-controls {
        height: 1;
        dock: top;
    }
    .wf-task-list {
        height: auto;
        max-height: 15;
        padding: 0 1;
    }
    """

    is_running: reactive[bool] = reactive(False)

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._tasks: List[Dict[str, Any]] = []
        self._state: str = "idle"

    def compose(self) -> ComposeResult:
        yield Static("[dim]No workflow active[/]", id="workflow-status")
        yield Static("", id="workflow-tasks", classes="wf-task-list")

    def update_event(self, data: Dict[str, Any]):
        """Process a single SSE workflow event."""
        try:
            status_widget = self.query_one("#workflow-status", Static)
            tasks_widget = self.query_one("#workflow-tasks", Static)
        except Exception:
            return

        event_type = data.get("type", "")

        if event_type == "state_changed" or "state" in data:
            self._state = data.get("state", data.get("new_state", "unknown"))
            self.is_running = self._state in ("running", "working")
            icon = "[green]▶[/]" if self.is_running else "[yellow]⏸[/]" if self._state == "paused" else "[dim]■[/]"
            status_widget.update(f"{icon} Workflow: {self._state}")

        elif event_type == "task_added" or "task" in data:
            task = data.get("task", data)
            task_info = {
                "id": task.get("task_id", task.get("id", "?")),
                "title": task.get("title", task.get("description", "?")),
                "status": task.get("status", "pending"),
            }
            existing = [t for t in self._tasks if t["id"] == task_info["id"]]
            if existing:
                existing[0].update(task_info)
            else:
                self._tasks.append(task_info)
            self._render_tasks(tasks_widget)

        elif event_type == "task_updated":
            task = data.get("task", data)
            tid = task.get("task_id", task.get("id"))
            for t in self._tasks:
                if t["id"] == tid:
                    t["status"] = task.get("status", t["status"])
                    break
            self._render_tasks(tasks_widget)

        elif event_type == "summary" or "summary" in data:
            summary = data.get("summary", str(data))
            status_widget.update(f"[cyan]Summary:[/] {str(summary)[:80]}")

        else:
            if isinstance(data, dict) and data:
                status_widget.update(f"[dim]event: {json.dumps(data)[:80]}[/]")

    def _render_tasks(self, widget: Static):
        if not self._tasks:
            widget.update("")
            return
        icons = {"pending": "[dim]○[/]", "running": "[yellow]●[/]", "done": "[green]✓[/]", "failed": "[red]✗[/]"}
        lines = []
        for t in self._tasks[-10:]:
            icon = icons.get(t["status"], "[dim]?[/]")
            lines.append(f"  {icon} {t['title']}")
        widget.update("\n".join(lines))

    def clear(self):
        self._tasks.clear()
        self._state = "idle"
        try:
            self.query_one("#workflow-status", Static).update("[dim]No workflow active[/]")
            self.query_one("#workflow-tasks", Static).update("")
        except Exception:
            pass


class Sidebar(Widget):
    """Collapsible sidebar container with all side panels."""

    DEFAULT_CSS = """
    Sidebar {
        width: 35;
        dock: left;
    }
    """

    def __init__(self, project_path: str = ".", **kwargs):
        super().__init__(**kwargs)
        self.project_path = project_path

    def compose(self) -> ComposeResult:
        yield VerticalScroll(
            Collapsible(
                FileTreePanel(self.project_path, id="file-tree-panel"),
                title="Files",
                collapsed=False,
            ),
            Collapsible(
                ToolStatusPanel(id="tool-status-panel"),
                title="Tools",
                collapsed=True,
            ),
            Collapsible(
                WorkflowPanel(id="workflow-panel"),
                title="Workflow",
                collapsed=True,
            ),
            id="sidebar-scroll",
        )
