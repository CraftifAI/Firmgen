"""Manages chat message state and processes incoming SSE stream data."""
from __future__ import annotations

import json
from typing import List, Dict, Any, Optional, Callable
from dataclasses import dataclass, field

from refact_tui.services.agent_client import Message, ToolCallDict, Usage, ChoiceDeltaCollector


@dataclass
class StreamState:
    """Holds the complete chat conversation state."""
    messages: List[Message] = field(default_factory=list)
    tool_calls: Dict[str, ToolCallDict] = field(default_factory=dict)
    streaming_tool_calls: List[ToolCallDict] = field(default_factory=list)
    is_streaming: bool = False
    current_content: str = ""
    last_usage: Optional[Usage] = None

    # Callbacks for UI updates
    on_content_delta: Optional[Callable[[str], None]] = None
    on_message: Optional[Callable[[Message], None]] = None
    on_tool_call_update: Optional[Callable[[List[ToolCallDict]], None]] = None
    on_stream_end: Optional[Callable[[], None]] = None
    on_usage_update: Optional[Callable[[Usage], None]] = None

    def clear(self):
        self.messages.clear()
        self.tool_calls.clear()
        self.streaming_tool_calls.clear()
        self.is_streaming = False
        self.current_content = ""
        self.last_usage = None

    def add_user_message(self, content: str):
        self.messages.append(Message(role="user", content=content))

    def process_chunk(self, data: Dict[str, Any], deltas: ChoiceDeltaCollector):
        """Process a single SSE chunk from the chat stream."""
        if "choices" in data:
            choices = data.get("choices", [])
            if not choices and data.get("usage"):
                u = Usage(**data["usage"])
                self.last_usage = u
                if self.on_usage_update:
                    self.on_usage_update(u)
                return

            delta = choices[0].get("delta", {})
            delta_content = delta.get("content")

            if delta.get("tool_calls"):
                self.streaming_tool_calls = list(deltas.choices[0].tool_calls or [])
                if self.on_tool_call_update:
                    self.on_tool_call_update(self.streaming_tool_calls)

            finish_reason = choices[0].get("finish_reason")
            if finish_reason == "tool_calls":
                for tc in self.streaming_tool_calls:
                    self.tool_calls[tc.id] = tc

            if delta_content:
                self.current_content += delta_content
                if self.on_content_delta:
                    self.on_content_delta(delta_content)

            if finish_reason == "stop":
                if self.on_content_delta:
                    self.on_content_delta("\n")

        elif "role" in data or isinstance(data, Message):
            self.streaming_tool_calls.clear()
            if isinstance(data, Message):
                msg = data
            else:
                msg = Message.model_validate(data)

            replace_last_user = False
            if msg.role in ("user", "system"):
                if self.messages and self.messages[-1].role == "user":
                    replace_last_user = True

            if replace_last_user:
                self.messages[-1] = msg
            else:
                self.messages.append(msg)

            if msg.role == "assistant" and msg.tool_calls:
                for tc in msg.tool_calls:
                    self.tool_calls[tc.id] = tc

            if self.on_message:
                self.on_message(msg)

        elif "subchat_id" in data:
            pass

        if u := data.get("usage") if isinstance(data, dict) else None:
            usage = Usage(**u)
            self.last_usage = usage
            if self.on_usage_update:
                self.on_usage_update(usage)

    def finalize_stream(self, result_messages: List[Message]):
        """Called when streaming completes. Syncs the messages list."""
        self.messages = result_messages
        self.is_streaming = False
        self.current_content = ""
        self.streaming_tool_calls.clear()
        if self.on_stream_end:
            self.on_stream_end()

    @property
    def has_pending_tool_calls(self) -> bool:
        if not self.messages:
            return False
        last = self.messages[-1]
        return last.role == "assistant" and bool(last.tool_calls)
