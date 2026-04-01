"""Diff viewer widget for displaying file changes."""
from __future__ import annotations

import json
from typing import List, Dict, Any

from textual.widget import Widget
from textual.app import ComposeResult
from textual.widgets import Static
from rich.syntax import Syntax
from rich.text import Text
from rich.panel import Panel


class DiffChunkWidget(Static):
    """Renders a single diff chunk with syntax highlighting."""

    def __init__(self, chunk: Dict[str, Any], **kwargs):
        super().__init__(**kwargs)
        self.chunk = chunk

    def render(self):
        fname = self.chunk.get("file_name", "unknown")
        l1 = self.chunk.get("line1", 0)
        l2 = self.chunk.get("line2", 0)

        lines = []
        lines.append(f"--- {fname}:{l1}-{l2}")
        if self.chunk.get("lines_remove"):
            for line in self.chunk["lines_remove"].splitlines():
                lines.append(f"-{line}")
        if self.chunk.get("lines_add"):
            for line in self.chunk["lines_add"].splitlines():
                lines.append(f"+{line}")

        diff_text = "\n".join(lines)
        return Syntax(diff_text, "diff", theme="monokai", line_numbers=False)


class DiffViewer(Widget):
    """Displays a collection of diff chunks."""

    DEFAULT_CSS = """
    DiffViewer { height: auto; }
    """

    def __init__(self, diff_content: str, **kwargs):
        super().__init__(**kwargs)
        self.diff_content = diff_content

    def compose(self) -> ComposeResult:
        try:
            chunks = json.loads(self.diff_content)
            if isinstance(chunks, list):
                for chunk in chunks:
                    yield DiffChunkWidget(chunk, classes="diff-chunk")
        except (json.JSONDecodeError, TypeError):
            yield Static(self.diff_content)
