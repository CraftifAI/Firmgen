"""Chat panel widget - displays conversation messages with streaming support."""
from __future__ import annotations

import json
import re
from typing import Optional, List, Tuple

from textual.app import ComposeResult
from textual.widget import Widget
from textual.widgets import Static, Markdown, Collapsible, Button
from textual.containers import VerticalScroll, Horizontal
from textual.reactive import reactive
from textual.message import Message as TextualMessage
from rich.text import Text
from rich.syntax import Syntax
from rich.panel import Panel
from rich.console import Group

from refact_tui.services.agent_client import Message, ToolCallDict

# Pattern: a line like "- [Some action text]"
_ACTION_RE = re.compile(r'^-\s*\[([^\]]+)\]\s*$', re.MULTILINE)


def parse_action_buttons(content: str) -> Tuple[str, List[str]]:
    """Split assistant content into (prose_text, [action_labels]).

    Lines matching ``- [Label]`` are extracted as action buttons.  The
    remaining text (with those lines removed) is returned as prose.
    """
    actions = _ACTION_RE.findall(content)
    if not actions:
        return content, []
    prose = _ACTION_RE.sub('', content).rstrip()
    return prose, actions


class ActionButtonsWidget(Widget):
    """Renders a row of clickable follow-up action buttons from /v1/links."""

    DEFAULT_CSS = """
    ActionButtonsWidget {
        height: auto;
        margin: 1 0 1 2;
    }
    ActionButtonsWidget Horizontal {
        height: auto;
    }
    ActionButtonsWidget Button {
        margin: 0 1 0 0;
        min-width: 18;
        height: 3;
    }
    """

    class ButtonAction(TextualMessage):
        """Fired when an action button is clicked."""
        def __init__(self, label: str) -> None:
            super().__init__()
            self.label = label

    def __init__(self, actions: List[str], **kwargs):
        super().__init__(**kwargs)
        self._actions = actions

    def compose(self) -> ComposeResult:
        with Horizontal():
            for label in self._actions:
                yield Button(label, classes="action-btn", variant="primary")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        event.stop()
        label = str(event.button.label)
        self.post_message(self.ButtonAction(label))


class ThinkingBlockWidget(Widget):
    """Renders a collapsible thinking block from reasoning models."""

    DEFAULT_CSS = """
    ThinkingBlockWidget {
        height: auto;
        margin: 0 0 0 2;
    }
    ThinkingBlockWidget .thinking-content {
        color: $text-muted;
    }
    """

    def __init__(self, blocks: List[dict], **kwargs):
        super().__init__(**kwargs)
        self._blocks = blocks

    def compose(self) -> ComposeResult:
        total_text = ""
        for b in self._blocks:
            if b.get("type") == "thinking" and b.get("thinking"):
                total_text += b["thinking"]

        if total_text:
            preview = total_text[:120].replace("\n", " ")
            if len(total_text) > 120:
                preview += "…"
            yield Collapsible(
                Static(total_text, classes="thinking-content"),
                title=f"💭 Thinking: {preview}",
                collapsed=True,
                classes="thinking-collapsible",
            )


class ToolCallWidget(Widget):
    """Displays a tool call with its name, arguments, and optional expandable result."""

    DEFAULT_CSS = """
    ToolCallWidget {
        height: auto;
        margin: 0 0 0 2;
        padding: 0 0 0 1;
        border-left: thick $warning;
    }
    .tool-call-header { height: auto; }
    .tool-result-content {
        height: auto;
        max-height: 20;
        color: $text-muted;
    }
    """

    def __init__(
        self,
        tool_call: ToolCallDict,
        result: Optional[str] = None,
        failed: bool = False,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self.tool_call = tool_call
        self.result = result
        self.failed = failed

    def compose(self) -> ComposeResult:
        name = getattr(self.tool_call.function, "name", "?")
        raw_args = getattr(self.tool_call.function, "arguments", None) or ""
        try:
            args = json.loads(raw_args) if isinstance(raw_args, str) else raw_args
            if isinstance(args, dict):
                args_str = ", ".join(f"{k}={repr(v)}" for k, v in args.items())
            else:
                args_str = str(raw_args)[:100]
        except (json.JSONDecodeError, AttributeError, TypeError):
            args_str = str(raw_args)[:100]

        if len(args_str) > 100:
            args_str = args_str[:97] + "…"

        if self.failed:
            status = "[red bold]✗ FAILED[/]"
        elif self.result is not None:
            status = "[green bold]✓ OK[/]"
        else:
            status = "[yellow bold]⏳ PENDING[/]"

        yield Static(
            Text.from_markup(f"  [cyan bold]🔧[/] {status} [bold]{name}[/]({args_str})"),
            classes="tool-call-header",
        )

        if self.result and len(self.result) > 0:
            preview = self.result[:500]
            if len(self.result) > 500:
                preview += f"\n… ({len(self.result)} chars total)"
            yield Collapsible(
                Static(preview, classes="tool-result-content"),
                title="Result",
                collapsed=True,
                classes="tool-result-collapsible",
            )


class DiffBlockWidget(Static):
    """Renders a diff chunk with syntax highlighting."""

    def __init__(self, chunk: dict, **kwargs):
        super().__init__(**kwargs)
        self.chunk = chunk

    def render(self):
        fname = self.chunk.get("file_name", "unknown")
        l1 = self.chunk.get("line1", 0)
        l2 = self.chunk.get("line2", 0)

        lines = [f"--- {fname}:{l1}-{l2}"]
        if self.chunk.get("lines_remove"):
            for line in self.chunk["lines_remove"].splitlines():
                lines.append(f"-{line}")
        if self.chunk.get("lines_add"):
            for line in self.chunk["lines_add"].splitlines():
                lines.append(f"+{line}")

        return Syntax("\n".join(lines), "diff", theme="monokai", line_numbers=False)


class SubchatWidget(Widget):
    """Renders a subchat (nested tool call conversation)."""

    DEFAULT_CSS = """
    SubchatWidget { height: auto; margin: 0 0 0 4; padding: 0; }
    """

    def __init__(self, subchat_id: str, messages: list, **kwargs):
        super().__init__(**kwargs)
        self.subchat_id = subchat_id
        self.subchat_messages = messages

    def compose(self) -> ComposeResult:
        summary_parts = []
        for m in self.subchat_messages:
            if m.role == "context_file" and isinstance(m.content, str):
                try:
                    files = json.loads(m.content)
                    for f in files:
                        summary_parts.append(f.get("file_name", "?"))
                except (json.JSONDecodeError, TypeError):
                    pass
            elif m.role == "assistant" and isinstance(m.content, str):
                summary_parts.append(m.content[:80])

        summary = ", ".join(summary_parts[:5])
        if len(summary_parts) > 5:
            summary += f" (+{len(summary_parts) - 5} more)"

        yield Collapsible(
            Static(summary or "[dim]no content[/]"),
            title=f"↳ subchat {self.subchat_id}",
            collapsed=True,
        )


class MessageWidget(Widget):
    """Renders a single chat message with all its components."""

    DEFAULT_CSS = """
    MessageWidget { height: auto; padding: 0; }
    """

    def __init__(self, msg: Message, tool_results: Optional[dict] = None, **kwargs):
        super().__init__(**kwargs)
        self.msg = msg
        self.tool_results = tool_results or {}

    def compose(self) -> ComposeResult:
        role = self.msg.role
        content = self.msg.content

        if role == "user":
            yield Static(Text.from_markup(f"\n[bold on dark_blue] 👤 You [/]"), classes="msg-role")
            if isinstance(content, str):
                yield Static(content, classes="msg-content user-msg")
            elif isinstance(content, list):
                text_parts = []
                for item in content:
                    if hasattr(item, "m_type"):
                        if item.m_type == "text":
                            text_parts.append(item.m_content)
                        else:
                            text_parts.append(f"[{item.m_type}]")
                    elif isinstance(item, dict):
                        if item.get("m_type") == "text":
                            text_parts.append(item.get("m_content", ""))
                        elif item.get("type") == "text":
                            text_parts.append(item.get("text", ""))
                        else:
                            text_parts.append(f"[{item.get('m_type', item.get('type', '?'))}]")
                yield Static(" ".join(text_parts), classes="msg-content user-msg")

        elif role == "assistant":
            yield Static(Text.from_markup(f"\n[bold on dark_green] 🤖 Assistant [/]"), classes="msg-role")

            # Render thinking blocks first (collapsible)
            if self.msg.thinking_blocks:
                yield ThinkingBlockWidget(self.msg.thinking_blocks)

            if isinstance(content, str) and content.strip():
                prose, actions = parse_action_buttons(content)
                if prose.strip():
                    yield Markdown(prose, classes="msg-content assistant-msg")
                if actions:
                    yield ActionButtonsWidget(actions, classes="action-buttons")
            if self.msg.tool_calls:
                for tc in self.msg.tool_calls:
                    result = self.tool_results.get(tc.id)
                    failed = False
                    result_text = None
                    if result:
                        result_text = result.get("content", "")
                        failed = result.get("failed", False)
                    yield ToolCallWidget(
                        tc,
                        result=result_text,
                        failed=failed,
                        classes="tool-call",
                    )

        elif role == "tool":
            tc_id = self.msg.tool_call_id
            failed = getattr(self.msg, "tool_failed", False)

            if self.msg.subchats:
                for sc_id, sc_msgs in self.msg.subchats.items():
                    yield SubchatWidget(sc_id, sc_msgs)

        elif role == "context_file":
            if isinstance(content, str):
                try:
                    files = json.loads(content)
                    for f in files:
                        fname = f.get("file_name", "?")
                        l1 = f.get("line1", 0)
                        l2 = f.get("line2", 0)
                        yield Static(
                            Text.from_markup(f"  [dim cyan]📎 attached[/] {fname}:{l1}-{l2}"),
                            classes="context-file",
                        )
                except json.JSONDecodeError:
                    yield Static(Text.from_markup(f"[dim]{content[:100]}[/]"))

        elif role == "diff":
            yield Static(Text.from_markup(f"\n[bold on dark_orange] 📝 Diff [/]"), classes="msg-role")
            if isinstance(content, str):
                try:
                    chunks = json.loads(content)
                    for chunk in chunks:
                        yield DiffBlockWidget(chunk, classes="diff-block")
                except json.JSONDecodeError:
                    yield Static(content, classes="msg-content")

        elif role == "system":
            if isinstance(content, str) and content.strip():
                short = content[:120] + ("…" if len(content) > 120 else "")
                yield Collapsible(
                    Static(content, classes="system-full"),
                    title=f"⚙ system: {short}",
                    collapsed=True,
                    classes="system-collapsible",
                )

        elif role in ("plain_text", "cd_instruction"):
            yield Static(
                Text.from_markup(f"[dim italic]{role}:[/] {str(content)[:200]}"),
                classes="msg-content",
            )


class StreamingContent(Static):
    """Live-updating widget that displays content as it streams in."""

    content_text: reactive[str] = reactive("", layout=True)
    tool_status: reactive[str] = reactive("", layout=True)

    def render(self):
        parts = []
        if self.content_text:
            parts.append(Text(self.content_text))
        elif not self.tool_status:
            parts.append(Text.from_markup("[dim italic]💭 thinking…[/]"))
        if self.tool_status:
            parts.append(Text.from_markup(f"\n[yellow]⏳ {self.tool_status}[/]"))
        if len(parts) == 0:
            return Text("")
        if len(parts) == 1:
            return parts[0]
        result = parts[0].copy()
        for p in parts[1:]:
            result.append_text(p)
        return result

    def append_text(self, text: str):
        self.content_text += text

    def set_tool_status(self, status: str):
        self.tool_status = status


class ChatPanel(Widget):
    """Main chat panel with scrollable message history and streaming support."""

    DEFAULT_CSS = """
    ChatPanel {
        height: 1fr;
    }
    """

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._streaming_widget: Optional[StreamingContent] = None
        self._messages_container: Optional[VerticalScroll] = None
        self._streaming_had_content: bool = False

    def compose(self) -> ComposeResult:
        yield VerticalScroll(id="chat-messages")

    def on_mount(self):
        self._messages_container = self.query_one("#chat-messages", VerticalScroll)

    async def add_message(self, msg: Message):
        container = self._messages_container
        if container is None:
            return
        await container.mount(MessageWidget(msg))
        container.scroll_end(animate=False)

    async def start_streaming(self):
        container = self._messages_container
        if container is None:
            return
        await container.mount(Static(Text.from_markup(f"\n[bold on dark_green] 🤖 Assistant [/]"), classes="msg-role"))
        self._streaming_widget = StreamingContent(classes="streaming-content")
        self._streaming_had_content = False
        await container.mount(self._streaming_widget)
        container.scroll_end(animate=False)

    def append_streaming_content(self, text: str):
        if self._streaming_widget:
            self._streaming_widget.append_text(text)
            self._streaming_had_content = True
            if self._messages_container:
                self._messages_container.scroll_end(animate=False)

    def update_streaming_tool(self, status: str):
        if self._streaming_widget:
            self._streaming_widget.set_tool_status(status)

    async def end_streaming(self, final_messages: list, had_error: bool = False):
        had_content = self._streaming_had_content
        if self._streaming_widget:
            await self._streaming_widget.remove()
            self._streaming_widget = None
            self._streaming_had_content = False
        # Only remove the temporary role label if no real content
        if not had_content and self._messages_container:
            role_widgets = self._messages_container.query(".msg-role")
            if role_widgets:
                last_role = role_widgets.last()
                try:
                    await last_role.remove()
                except Exception:
                    pass

    async def show_error(self, message: str):
        """Display an inline error message in the chat panel."""
        container = self._messages_container
        if container is None:
            return
        await container.mount(
            Static(Text.from_markup(f"[bold red]⚠ Error:[/] [dim]{message}[/]"),
                   classes="msg-content")
        )
        container.scroll_end(animate=False)

    async def show_links(self, links: list):
        """Render follow-up links from /v1/links as clickable action buttons."""
        follow_ups = [
            lnk["link_text"]
            for lnk in links
            if lnk.get("link_action") == "follow-up" and lnk.get("link_text")
        ]
        if not follow_ups:
            return
        container = self._messages_container
        if container is None:
            return
        await container.mount(ActionButtonsWidget(follow_ups, classes="action-buttons"))
        container.scroll_end(animate=False)

    async def clear_messages(self):
        container = self._messages_container
        if container:
            await container.remove_children()

    async def render_all_messages(self, messages: list):
        """Re-render all messages, building a tool result lookup for assistant messages."""
        await self.clear_messages()
        container = self._messages_container
        if container is None:
            return

        tool_results: dict = {}
        for msg in messages:
            if msg.role == "tool":
                tool_results[msg.tool_call_id] = {
                    "content": msg.content if isinstance(msg.content, str) else str(msg.content),
                    "failed": getattr(msg, "tool_failed", False),
                }

        for msg in messages:
            if msg.role == "system" and not (isinstance(msg.content, str) and msg.content.strip()):
                continue
            if msg.role == "tool":
                # Tool results are shown inline with the assistant's tool calls
                if msg.subchats:
                    await container.mount(MessageWidget(msg))
                continue
            widget = MessageWidget(msg, tool_results=tool_results)
            await container.mount(widget)

        container.scroll_end(animate=False)
