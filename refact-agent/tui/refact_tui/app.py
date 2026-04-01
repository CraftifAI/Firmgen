"""Refact Agent TUI - Main Application."""
from __future__ import annotations

import argparse
import asyncio
import os
import random
import sys
from pathlib import Path
from typing import Optional

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical
from textual.widgets import Static, Header, Footer
from textual import work, events
from rich.text import Text

from refact_tui.services.agent_client import AgentClient, LSPRunner, Message, Caps
from refact_tui.services.stream_handler import StreamState
from refact_tui.widgets.chat_panel import ChatPanel, ActionButtonsWidget
from refact_tui.widgets.input_area import ChatInput
from refact_tui.widgets.status_bar import StatusBar
from refact_tui.widgets.sidebar import Sidebar, WorkflowPanel
from refact_tui.screens.chat import HelpScreen
from refact_tui.screens.settings import SettingsScreen


class RefactTUI(App):
    """Terminal UI dashboard for Refact Agent."""

    TITLE = "Refact Agent"
    CSS_PATH = "styles/app.tcss"

    BINDINGS = [
        Binding("ctrl+q", "quit", "Quit", priority=True),
        Binding("ctrl+b", "toggle_sidebar", "Sidebar", priority=True),
        Binding("ctrl+l", "clear_chat", "Clear", priority=True),
        Binding("ctrl+n", "new_chat", "New Chat", priority=True),
        Binding("ctrl+m", "switch_model", "Model", priority=True),
        Binding("ctrl+p", "workflow_pause_resume", "Pause/Resume", priority=True),
        Binding("f1", "show_help", "Help"),
        Binding("escape", "cancel_stream", "Stop", priority=True),
    ]

    def __init__(
        self,
        base_url: Optional[str] = None,
        project_path: str = ".",
        model: Optional[str] = None,
        lsp_runner: Optional[LSPRunner] = None,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self._base_url = base_url
        self._project_path = os.path.abspath(project_path)
        self._model_override = model
        self._lsp_runner = lsp_runner
        self._client: Optional[AgentClient] = None
        self._state = StreamState()
        self._caps: Optional[Caps] = None
        self._model: str = ""
        self._chat_id: str = f"tui-{random.randint(0, 0xFFFFFFFF):08x}"
        self._sidebar_visible = True
        self._help_visible = False
        self._streaming_task: Optional[asyncio.Task] = None

    def compose(self) -> ComposeResult:
        yield Static(
            Text.from_markup("[bold]Refact Agent[/] │ [dim]^B sidebar │ ^M model │ ^N new │ ^L clear │ ^P pause │ F1 help │ Esc stop │ ^Q quit[/]"),
            id="app-header",
        )
        yield Horizontal(
            Sidebar(self._project_path, id="sidebar"),
            Vertical(
                ChatPanel(id="chat-panel"),
                ChatInput(id="chat-input"),
                id="chat-area",
            ),
            id="main-content",
        )
        yield StatusBar(id="status-bar")

    async def on_mount(self):
        status = self.query_one("#status-bar", StatusBar)
        status.project_path = self._project_path
        self._connect_agent()

    @work(exclusive=True)
    async def _connect_agent(self):
        status = self.query_one("#status-bar", StatusBar)

        if self._base_url:
            self._client = AgentClient(self._base_url)
        elif self._lsp_runner:
            self._client = AgentClient(self._lsp_runner.base_url())
        else:
            status.model_name = "no agent"
            return

        for attempt in range(30):
            if await self._client.ping():
                break
            await asyncio.sleep(1)
        else:
            status.model_name = "connection failed"
            return

        try:
            self._caps = await self._client.fetch_caps()
            if self._model_override and self._model_override in self._caps.chat_models:
                self._model = self._model_override
            else:
                self._model = self._caps.chat_default_model
            status.model_name = self._model
        except Exception as e:
            status.model_name = f"error: {e}"
            return

        self._poll_rag_status()
        self._listen_workflow_events()

    @work(exclusive=False, thread=False)
    async def _listen_workflow_events(self):
        """Subscribe to workflow SSE events and update the workflow panel."""
        if not self._client:
            return
        try:
            wf_panel = self.query_one("#workflow-panel", WorkflowPanel)
        except Exception:
            return
        while True:
            try:
                async for event in self._client.workflow_events():
                    wf_panel.update_event(event)
            except Exception:
                pass
            await asyncio.sleep(5)

    @work(exclusive=False, thread=False)
    async def _poll_rag_status(self):
        """Periodically poll RAG status and update the status bar."""
        status = self.query_one("#status-bar", StatusBar)
        while True:
            if self._client:
                try:
                    rag = await self._client.fetch_rag_status()
                    status.update_rag_status(rag)
                except Exception:
                    pass
            await asyncio.sleep(3)

    async def on_action_buttons_widget_button_action(
        self, event: "ActionButtonsWidget.ButtonAction"
    ) -> None:
        """Handle a click on an agent-generated action button."""
        # Submit the button label as if the user typed it.
        await self.on_chat_input_submitted(ChatInput.Submitted(event.label))

    async def on_chat_input_submitted(self, event: ChatInput.Submitted):
        text = event.text

        if text.lower() in ("exit", "quit", "/exit", "/quit"):
            self.exit()
            return

        if text.startswith("/"):
            await self._handle_command(text)
            return

        if not self._client:
            self.notify("Not connected to agent", severity="error")
            return

        chat_panel = self.query_one("#chat-panel", ChatPanel)
        status = self.query_one("#status-bar", StatusBar)
        chat_input = self.query_one("#chat-input", ChatInput)

        self._state.add_user_message(text)
        await chat_panel.add_message(Message(role="user", content=text))

        chat_input.set_disabled(True)
        status.is_streaming = True
        await chat_panel.start_streaming()

        self._streaming_task = asyncio.create_task(self._run_chat_loop())

    async def _run_chat_loop(self, max_resubmit: int = 6):
        chat_panel = self.query_one("#chat-panel", ChatPanel)
        status = self.query_one("#status-bar", StatusBar)
        chat_input = self.query_one("#chat-input", ChatInput)
        tool_panel = None
        try:
            tool_panel = self.query_one("#tool-status-panel")
        except Exception:
            pass

        try:
            for step in range(max_resubmit):
                self._state.is_streaming = True

                def on_data(data, deltas):
                    if "choices" in data:
                        choices = data.get("choices", [])
                        if choices:
                            delta = choices[0].get("delta", {})
                            if delta_content := delta.get("content"):
                                chat_panel.append_streaming_content(delta_content)
                            if delta.get("tool_calls") and deltas:
                                tc_list = list(deltas.choices[0].tool_calls or [])
                                if tc_list:
                                    last_tc = tc_list[-1]
                                    args_preview = (getattr(last_tc.function, "arguments", None) or "")[:80]
                                    chat_panel.update_streaming_tool(
                                        f"calling {getattr(last_tc.function, 'name', '?')}({args_preview})"
                                    )
                    self._state.process_chunk(data, deltas)

                messages_before = len(self._state.messages)
                result_messages = await self._client.chat_stream(
                    messages=self._state.messages,
                    model=self._model,
                    chat_id=self._chat_id,
                    on_data=on_data,
                )

                # Detect error: agent returned nothing new (same or fewer messages,
                # no new assistant content). This happens when OpenAI rejects the
                # request with a 4xx error that the agent propagates silently.
                new_msgs = [m for m in result_messages if m not in self._state.messages]
                got_content = any(
                    m.role == "assistant" and (m.content or m.tool_calls)
                    for m in new_msgs
                )
                had_error = not got_content and len(result_messages) <= messages_before

                await chat_panel.end_streaming(result_messages, had_error=had_error)
                self._state.finalize_stream(result_messages)

                if had_error:
                    await chat_panel.show_error(
                        "Agent returned an empty response — the backend may have rejected the request. "
                        "Check server logs for details."
                    )
                    break

                if self._state.last_usage:
                    status.update_usage(self._state.last_usage)

                await chat_panel.render_all_messages(
                    [m for m in self._state.messages if m.role not in ("system",)]
                )

                if tool_panel and hasattr(tool_panel, 'add_tool_call'):
                    last = self._state.messages[-1] if self._state.messages else None
                    if last and last.role == "assistant" and last.tool_calls:
                        for tc in last.tool_calls:
                            tool_panel.add_tool_call(tc.function.name, "ok")

                if not self._state.has_pending_tool_calls:
                    # Fetch follow-up links from /v1/links (same as GUI does)
                    links = await self._client.fetch_links(
                        chat_id=self._chat_id,
                        messages=self._state.messages,
                        model=self._model,
                        chat_mode="AGENT",
                    )
                    if links:
                        await chat_panel.show_links(links)
                    break

                await chat_panel.start_streaming()

        except asyncio.CancelledError:
            await chat_panel.end_streaming([])
        except Exception as e:
            await chat_panel.end_streaming([])
            await chat_panel.show_error(str(e))
        finally:
            self._state.is_streaming = False
            status.is_streaming = False
            chat_input.set_disabled(False)
            chat_input.focus_input()
            self._streaming_task = None

    async def _handle_command(self, text: str):
        parts = text.split()
        cmd = parts[0].lower()

        if cmd == "/clear":
            await self.action_clear_chat()
        elif cmd == "/model":
            if len(parts) > 1 and self._caps:
                new_model = parts[1]
                if new_model in self._caps.chat_models:
                    self._model = new_model
                    status = self.query_one("#status-bar", StatusBar)
                    status.model_name = self._model
                    self.notify(f"Switched to {self._model}")
                else:
                    available = ", ".join(self._caps.chat_models.keys())
                    self.notify(f"Unknown model. Available: {available}", severity="warning")
            else:
                await self.action_switch_model()
        elif cmd == "/new":
            await self.action_new_chat()
        elif cmd == "/help":
            self.action_show_help()
        elif cmd == "/export":
            await self._export_chat()
        elif cmd == "/pause":
            await self.action_workflow_pause_resume()
        else:
            self.notify(f"Unknown command: {cmd}. Try /help for available commands.", severity="warning")

    async def _export_chat(self):
        """Export current chat history to a JSON file."""
        import json
        import time
        if not self._state.messages:
            self.notify("No messages to export", severity="warning")
            return
        export_dir = os.path.expanduser("~/.cache/refact/")
        os.makedirs(export_dir, exist_ok=True)
        filename = os.path.join(export_dir, f"tui_export_{int(time.time())}.json")
        msgs = [m.model_dump(exclude_none=True) for m in self._state.messages]
        with open(filename, "w") as f:
            json.dump(msgs, f, indent=2)
        self.notify(f"Exported to {filename}")

    def action_toggle_sidebar(self):
        sidebar = self.query_one("#sidebar", Sidebar)
        self._sidebar_visible = not self._sidebar_visible
        sidebar.display = self._sidebar_visible

    async def action_clear_chat(self):
        chat_panel = self.query_one("#chat-panel", ChatPanel)
        await chat_panel.clear_messages()
        self._state.clear()
        self.notify("Chat cleared")

    async def action_new_chat(self):
        await self.action_clear_chat()
        self._chat_id = f"tui-{random.randint(0, 0xFFFFFFFF):08x}"
        self.notify("New chat started")

    def action_show_help(self):
        self.push_screen(HelpScreen())

    async def action_switch_model(self):
        if not self._caps:
            self.notify("No models loaded yet", severity="warning")
            return

        def on_result(result: Optional[str]):
            if result:
                self._model = result
                status = self.query_one("#status-bar", StatusBar)
                status.model_name = self._model
                self.notify(f"Switched to {self._model}")

        self.push_screen(SettingsScreen(caps=self._caps, current_model=self._model), on_result)

    async def action_workflow_pause_resume(self):
        if not self._client:
            self.notify("Not connected to agent", severity="error")
            return
        try:
            wf_panel = self.query_one("#workflow-panel", WorkflowPanel)
            if wf_panel.is_running:
                await self._client.workflow_action("pause")
                self.notify("Workflow paused")
            else:
                await self._client.workflow_action("resume")
                self.notify("Workflow resumed")
        except Exception as e:
            self.notify(f"Workflow action failed: {e}", severity="error")

    def action_cancel_stream(self):
        if self._streaming_task and not self._streaming_task.done():
            self._streaming_task.cancel()
            self.notify("Streaming cancelled")

    async def action_quit(self):
        if self._client:
            await self._client.close()
        self.exit()


def parse_args():
    parser = argparse.ArgumentParser(description="Refact Agent TUI Dashboard")
    parser.add_argument("path_to_project", type=str, nargs="?", default=".", help="Path to the project")
    parser.add_argument("--port", type=int, default=None, help="Connect to an already-running agent on this port")
    parser.add_argument("--model", type=str, default=None, help="Model to use for chat")
    parser.add_argument("--lsp-binary", type=str, default=None, help="Path to refact-lsp binary (starts a new agent)")
    parser.add_argument("--address-url", type=str, default=None, help="Address URL for refact-lsp")
    parser.add_argument("--api-key", type=str, default=None, help="API key for refact-lsp")
    return parser.parse_args()


async def run_app():
    args = parse_args()
    project = os.path.abspath(args.path_to_project)

    if args.port:
        base_url = f"http://127.0.0.1:{args.port}/v1"
        app = RefactTUI(base_url=base_url, project_path=project, model=args.model)
        await app.run_async()
    elif args.lsp_binary:
        extra = []
        if args.address_url:
            extra.extend(["--address-url", args.address_url])
        if args.api_key:
            extra.extend(["--api-key", args.api_key])
        extra.extend(["--workspace-folder", project])
        runner = LSPRunner(args.lsp_binary, extra)
        async with runner:
            app = RefactTUI(
                base_url=runner.base_url(),
                project_path=project,
                model=args.model,
                lsp_runner=runner,
            )
            await app.run_async()
    else:
        # Try default port, or look for refact-lsp in common locations
        binary = None
        for candidate in [
            os.path.expanduser("~/.local/bin/refact-lsp"),
            "/usr/local/bin/refact-lsp",
            os.path.join(os.path.dirname(__file__), "..", "..", "..", "engine", "python_binding_and_cmdline", "refact", "bin", "refact-lsp"),
        ]:
            if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
                binary = candidate
                break

        if binary:
            extra = ["--workspace-folder", project]
            if args.address_url:
                extra.extend(["--address-url", args.address_url])
            if args.api_key:
                extra.extend(["--api-key", args.api_key])
            runner = LSPRunner(binary, extra)
            async with runner:
                app = RefactTUI(
                    base_url=runner.base_url(),
                    project_path=project,
                    model=args.model,
                    lsp_runner=runner,
                )
                await app.run_async()
        else:
            # Try connecting to default port
            app = RefactTUI(
                base_url="http://127.0.0.1:8001/v1",
                project_path=project,
                model=args.model,
            )
            await app.run_async()


def main():
    asyncio.run(run_app())


if __name__ == "__main__":
    main()
