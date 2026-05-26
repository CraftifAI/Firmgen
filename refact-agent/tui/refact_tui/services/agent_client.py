"""HTTP + SSE client for communicating with the Refact Agent engine."""
from __future__ import annotations

import json
import asyncio
import uuid
import os
import platform
import random
import subprocess
from typing import Optional, List, Dict, Any, Callable, Literal, DefaultDict, Union, AsyncIterator
from collections import defaultdict
from dataclasses import dataclass, field

import aiohttp
from pydantic import BaseModel, ConfigDict


# ── Data Models ────────────────────────────────────────────────────────────────

class FunctionDict(BaseModel):
    arguments: str
    name: str


class ToolCallDict(BaseModel):
    id: str
    function: FunctionDict
    type: str


class Usage(BaseModel):
    prompt_tokens: int = 0
    completion_tokens: int = 0
    cache_creation_input_tokens: int = 0
    cache_read_input_tokens: int = 0


class Message(BaseModel):
    role: Literal["system", "assistant", "user", "tool", "context_file", "diff", "plain_text", "cd_instruction"]
    content: Optional[Union[str, list]] = None
    tool_calls: Optional[List[ToolCallDict]] = None
    finish_reason: str = ""
    tool_call_id: str = ""
    usage: Optional[Usage] = None
    subchats: Optional[DefaultDict[str, list]] = None
    thinking_blocks: Optional[List[Dict]] = None
    model_config = ConfigDict()


class CapsModel(BaseModel):
    n_ctx: int
    supports_tools: bool


class Caps(BaseModel):
    chat_models: Dict[str, CapsModel]
    chat_default_model: str


# ── Tool Group Models ──────────────────────────────────────────────────────────

class ToolSource(BaseModel):
    source_type: str = "builtin"
    config_path: str = ""


class ToolSpec(BaseModel):
    name: str = ""
    display_name: str = ""
    description: str = ""
    parameters: List[Dict[str, Any]] = []
    source: Optional[ToolSource] = None
    agentic: bool = False
    experimental: bool = False


class ToolEntry(BaseModel):
    spec: ToolSpec
    enabled: bool = True


class ToolGroup(BaseModel):
    name: str
    category: str = "builtin"
    description: str = ""
    tools: List[ToolEntry] = []


class ToolGroupUpdate(BaseModel):
    name: str
    source: ToolSource
    enabled: bool


# ── Tool Confirmation Models ───────────────────────────────────────────────────

class ConfirmationPauseReason(BaseModel):
    type: str  # "confirmation" or "denial"
    command: str = ""
    rule: str = ""
    tool_call_id: str = ""
    integr_config_path: Optional[str] = None


class ConfirmationResponse(BaseModel):
    pause: bool = False
    pause_reasons: List[ConfirmationPauseReason] = []


# ── SSE Helpers ────────────────────────────────────────────────────────────────

async def _iter_sse_events(response: aiohttp.ClientResponse) -> AsyncIterator[str]:
    """Yield complete SSE data payloads from an HTTP response.

    Correctly handles:
    - Events spanning multiple HTTP chunks
    - Multiple events in a single HTTP chunk
    - Empty-line event delimiters per SSE spec
    - `data:` prefix stripping
    """
    buffer = ""
    async for raw_chunk in response.content:
        buffer += raw_chunk.decode("utf-8", errors="replace")
        # Process all complete lines in the buffer
        while "\n" in buffer:
            line, buffer = buffer.split("\n", 1)
            line = line.rstrip("\r")
            if not line:
                # Empty line = event boundary (SSE spec), but we yield per data line
                continue
            if line.startswith("data: "):
                payload = line[6:]
                if payload == "[DONE]":
                    return
                yield payload
            # Ignore other SSE fields (event:, id:, retry:, comments)

    # Handle any remaining data in buffer (no trailing newline)
    if buffer.strip():
        line = buffer.strip()
        if line.startswith("data: "):
            payload = line[6:]
            if payload != "[DONE]":
                yield payload


# ── Delta Collector ────────────────────────────────────────────────────────────

class ChoiceDeltaCollector:
    def __init__(self, n_answers: int):
        self.n_answers = n_answers
        self.choices: List[Message] = [Message(role="assistant", content="") for _ in range(n_answers)]

    def add_deltas(self, j_choices: List[Dict[str, Any]]):
        for j_choice in j_choices:
            j_index = j_choice["index"]
            if j_index < 0 or j_index >= self.n_answers:
                continue
            choice = self.choices[j_index]
            delta = j_choice["delta"]
            if (j_tool_calls := delta.get("tool_calls", None)) is not None:
                for plus_tool in j_tool_calls:
                    tool_idx = plus_tool["index"]
                    if choice.tool_calls is None:
                        choice.tool_calls = []
                    while len(choice.tool_calls) <= tool_idx:
                        choice.tool_calls.append(ToolCallDict(id="", function=FunctionDict(arguments="", name=""), type=""))
                    tool = choice.tool_calls[tool_idx]
                    if (i := plus_tool.get("id")) is not None and isinstance(i, str):
                        tool.id = i
                    if (t := plus_tool.get("type")) is not None and isinstance(t, str):
                        tool.type = t
                    if (function_plus := plus_tool.get("function")) is not None:
                        if (n := function_plus.get("name")) is not None and isinstance(n, str):
                            tool.function.name += n
                        if (a := function_plus.get("arguments")) is not None and isinstance(a, str):
                            tool.function.arguments += a
            elif plus_content := delta.get("content"):
                choice.content += plus_content
            elif "finish_reason" in j_choice:
                choice.finish_reason = j_choice.get("finish_reason", "")
            # Collect thinking blocks
            if "thinking_blocks" in delta:
                if choice.thinking_blocks is None:
                    choice.thinking_blocks = []
                for tb in delta["thinking_blocks"]:
                    choice.thinking_blocks.append(tb)


# ── LSP Runner ─────────────────────────────────────────────────────────────────

IS_WIN = platform.system() == "Windows"


class LSPRunner:
    """Manages the refact-lsp subprocess lifecycle."""

    def __init__(self, binary_path: str, extra_args: Optional[List[str]] = None):
        self._binary = binary_path
        self._extra_args = extra_args or []
        self._process: Optional[asyncio.subprocess.Process] = None
        self._port: int = 0
        self._stderr_task: Optional[asyncio.Task] = None

    @property
    def port(self) -> int:
        return self._port

    def base_url(self) -> str:
        return f"http://127.0.0.1:{self._port}/v1"

    async def start(self):
        for _ in range(5):
            self._port = random.randint(8100, 9100)
            args = [
                *self._extra_args,
                "--logs-stderr",
                f"--http-port={self._port}",
            ]

            if IS_WIN:
                # On Windows, use CREATE_NO_WINDOW to avoid popping a console
                self._process = await asyncio.create_subprocess_exec(
                    self._binary, *args,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE,
                    limit=1024 * 1024 * 64,
                    creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, 'CREATE_NO_WINDOW') else 0,
                )
            else:
                self._process = await asyncio.create_subprocess_exec(
                    self._binary, *args,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE,
                    limit=1024 * 1024 * 64,
                )

            listening = False
            port_busy = False
            while True:
                line = await self._read_stderr_line()
                if line is None:
                    if self._process.returncode is not None:
                        raise RuntimeError("LSP server exited unexpectedly")
                    await asyncio.sleep(0.1)
                    continue
                if "HTTP server listening" in line:
                    listening = True
                    break
                if "PORT_BUSY" in line:
                    port_busy = True
                    break
            if port_busy:
                await self._kill()
                continue
            if listening:
                self._stderr_task = asyncio.create_task(self._drain_stderr())
                return
        raise RuntimeError("Could not find a free port for refact-lsp")

    async def _read_stderr_line(self) -> Optional[str]:
        if self._process is None or self._process.stderr.at_eof():
            return None
        try:
            line = await asyncio.wait_for(self._process.stderr.readline(), timeout=0.1)
            return line.decode()
        except asyncio.TimeoutError:
            return None

    async def _drain_stderr(self):
        while self._process and self._process.returncode is None:
            await self._read_stderr_line()
            await asyncio.sleep(0.1)

    async def _kill(self):
        if self._process:
            self._process.terminate()
            try:
                await asyncio.wait_for(self._process.wait(), timeout=10)
            except asyncio.TimeoutError:
                self._process.kill()
                await self._process.wait()
            self._process = None

    async def stop(self):
        if self._stderr_task:
            self._stderr_task.cancel()
        await self._kill()

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, *exc):
        await self.stop()


# ── Port Detection ─────────────────────────────────────────────────────────────

def detect_default_port() -> int:
    """Detect the best default port to connect to.

    If the CraftifAI desktop app is likely running (port 8486), use that.
    Otherwise fall back to the standard refact-lsp port (8001).
    """
    import socket
    for port in (8486, 8001):
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.3):
                return port
        except (ConnectionRefusedError, OSError, socket.timeout):
            continue
    # Default to CraftifAI desktop port
    return 8486


# ── Agent Client ───────────────────────────────────────────────────────────────

class AgentClient:
    """High-level client for the Refact Agent HTTP API."""

    def __init__(self, base_url: str):
        self.base_url = base_url
        self._session: Optional[aiohttp.ClientSession] = None

    async def _get_session(self) -> aiohttp.ClientSession:
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=5000))
        return self._session

    async def close(self):
        if self._session and not self._session.closed:
            await self._session.close()

    async def ping(self) -> bool:
        try:
            session = await self._get_session()
            async with session.get(f"{self.base_url}/ping") as resp:
                return resp.status == 200
        except Exception:
            return False

    async def fetch_caps(self) -> Caps:
        session = await self._get_session()
        async with session.get(f"{self.base_url}/caps") as resp:
            data = await resp.json()
            return Caps(**data)

    async def fetch_tools(self) -> List[Dict[str, Any]]:
        session = await self._get_session()
        async with session.get(f"{self.base_url}/tools", timeout=aiohttp.ClientTimeout(total=40)) as resp:
            text = await resp.text()
            if resp.status != 200:
                return []
            return json.loads(text)

    async def fetch_tool_groups(self) -> List[ToolGroup]:
        """Fetch tool groups with their enabled/disabled state."""
        raw = await self.fetch_tools()
        groups = []
        for item in raw:
            try:
                groups.append(ToolGroup(**item))
            except Exception:
                continue
        return groups

    async def update_tool_groups(self, updates: List[Dict[str, Any]]) -> bool:
        """Update tool group enabled/disabled state via POST /v1/tools."""
        session = await self._get_session()
        try:
            async with session.post(
                f"{self.base_url}/tools",
                json={"tools": updates},
            ) as resp:
                return resp.status == 200
        except Exception:
            return False

    async def fetch_rag_status(self) -> Dict[str, Any]:
        try:
            session = await self._get_session()
            async with session.get(f"{self.base_url}/rag-status") as resp:
                return await resp.json(content_type=None)
        except Exception:
            return {"detail": "Unavailable"}

    async def at_command_completion(self, query: str, cursor: int, top_n: int = 6) -> Dict[str, Any]:
        session = await self._get_session()
        async with session.post(f"{self.base_url}/at-command-completion", json={
            "query": query,
            "cursor": cursor,
            "top_n": top_n,
        }) as resp:
            return await resp.json()

    async def fetch_models(self) -> List[Dict[str, Any]]:
        try:
            session = await self._get_session()
            async with session.get(f"{self.base_url}/models") as resp:
                return await resp.json()
        except Exception:
            return []

    async def check_tool_confirmation(
        self,
        tool_calls: List[ToolCallDict],
        messages: List[Message],
    ) -> ConfirmationResponse:
        """Check if any tool calls require user confirmation before execution.

        Calls POST /v1/tools-check-if-confirmation-needed.
        """
        session = await self._get_session()
        msgs_dicts = [m.model_dump(exclude_none=True) for m in messages]
        tc_dicts = [tc.model_dump(exclude_none=True) for tc in tool_calls]
        try:
            async with session.post(
                f"{self.base_url}/tools-check-if-confirmation-needed",
                json={"tool_calls": tc_dicts, "messages": msgs_dicts},
            ) as resp:
                if resp.status != 200:
                    return ConfirmationResponse(pause=False, pause_reasons=[])
                data = await resp.json(content_type=None)
                return ConfirmationResponse(**data)
        except Exception:
            return ConfirmationResponse(pause=False, pause_reasons=[])

    async def fetch_checkpoints_preview(self, chat_id: str) -> List[Dict[str, Any]]:
        """Fetch checkpoint list via POST /v1/checkpoints-preview."""
        session = await self._get_session()
        try:
            async with session.post(
                f"{self.base_url}/checkpoints-preview",
                json={"chat_id": chat_id},
            ) as resp:
                if resp.status != 200:
                    return []
                data = await resp.json(content_type=None)
                return data if isinstance(data, list) else data.get("checkpoints", [])
        except Exception:
            return []

    async def restore_checkpoint(self, chat_id: str, checkpoint_id: str) -> bool:
        """Restore a checkpoint via POST /v1/checkpoints-restore."""
        session = await self._get_session()
        try:
            async with session.post(
                f"{self.base_url}/checkpoints-restore",
                json={"chat_id": chat_id, "checkpoint_id": checkpoint_id},
            ) as resp:
                return resp.status == 200
        except Exception:
            return False

    async def fetch_links(
        self,
        chat_id: str,
        messages: List[Message],
        model: str,
        chat_mode: str = "AGENT",
    ) -> List[Dict[str, Any]]:
        """Call POST /v1/links to get follow-up action buttons."""
        try:
            msgs_dicts = [m.model_dump(exclude_none=True) for m in messages]
            payload = {
                "meta": {
                    "chat_id": chat_id,
                    "chat_mode": chat_mode,
                },
                "messages": msgs_dicts,
                "model_name": model,
            }
            session = await self._get_session()
            base = self.base_url.rstrip("/v1").rstrip("/")
            async with session.post(f"{base}/v1/links", json=payload) as resp:
                if resp.status != 200:
                    return []
                data = await resp.json(content_type=None)
                return data.get("links", [])
        except Exception:
            return []

    async def chat_stream(
        self,
        messages: List[Message],
        model: str,
        chat_id: str,
        *,
        max_tokens: int = 4096,
        temperature: float = 0.3,
        chat_mode: str = "AGENT",
        esp32_projects_path: Optional[str] = None,
        checkpoints_enabled: bool = True,
        boost_reasoning: bool = False,
        on_data: Optional[Callable[[Dict[str, Any], ChoiceDeltaCollector], None]] = None,
    ) -> List[Message]:
        """Send a chat request with streaming; calls on_data for every SSE event."""
        msgs_dicts = []
        for m in messages:
            d: Dict[str, Any] = {"role": m.role, "content": m.content}
            if m.role == "assistant":
                if m.tool_calls:
                    d["tool_calls"] = [tc.model_dump(exclude_none=True) for tc in m.tool_calls]
                if m.finish_reason:
                    d["finish_reason"] = m.finish_reason
            elif m.role == "tool":
                d["tool_call_id"] = m.tool_call_id
            msgs_dicts.append(d)

        meta: Dict[str, Any] = {
            "chat_id": chat_id,
            "chat_mode": chat_mode,
        }
        if esp32_projects_path and esp32_projects_path.strip():
            meta["esp32_projects_path"] = esp32_projects_path.strip()

        post_me: Dict[str, Any] = {
            "model": model,
            "messages": msgs_dicts,
            "temperature": temperature,
            "stream": True,
            "max_tokens": max_tokens,
            "only_deterministic_messages": False,
            "checkpoints_enabled": checkpoints_enabled,
            "meta": meta,
        }

        if boost_reasoning:
            post_me["parameters"] = {"boost_reasoning": True}

        deterministic: List[Message] = []
        subchats: DefaultDict[str, List[Message]] = defaultdict(list)
        deltas = ChoiceDeltaCollector(1)
        have_usage: Optional[Usage] = None

        session = await self._get_session()
        async with session.post(f"{self.base_url}/chat", json=post_me) as resp:
            async for payload in _iter_sse_events(resp):
                try:
                    j = json.loads(payload)
                except json.JSONDecodeError:
                    continue

                if "choices" in j and len(j.get("choices", [])) > 0:
                    if u := j.get("usage"):
                        new_u = Usage(**u)
                        have_usage = self._merge_usage(have_usage, new_u)
                    deltas.add_deltas(j["choices"])
                elif "role" in j:
                    deterministic.append(Message(**j))
                elif "subchat_id" in j:
                    key = j["tool_call_id"] + "__" + j["subchat_id"]
                    subchats[key].append(Message(**j["add_message"]))
                elif j.get("usage") is not None:
                    new_u = Usage(**j["usage"])
                    have_usage = self._merge_usage(have_usage, new_u)

                if on_data:
                    on_data(j, deltas)

        for c in deltas.choices:
            if c.content is not None and len(c.content) == 0:
                c.content = None
            c.usage = have_usage

        has_home = set()
        for d in deterministic:
            if d.tool_call_id:
                if subchats:
                    d.subchats = defaultdict(list)
                    for k, msglist in subchats.items():
                        if k.startswith(d.tool_call_id + "__"):
                            subchat_id = k[len(d.tool_call_id + "__"):]
                            d.subchats[subchat_id] = msglist
                            has_home.add(k)

        result = list(messages)
        if deterministic and deterministic[-1].role == "user":
            while result and result[-1].role == "user":
                result.pop()
        result.extend(deterministic)
        choice = deltas.choices[0]
        if choice.content is not None or choice.tool_calls is not None:
            result.append(choice)
        return result

    @staticmethod
    def _merge_usage(existing: Optional[Usage], new: Usage) -> Usage:
        if existing is None:
            return new
        return Usage(
            prompt_tokens=max(existing.prompt_tokens, new.prompt_tokens),
            completion_tokens=max(existing.completion_tokens, new.completion_tokens),
            cache_creation_input_tokens=max(existing.cache_creation_input_tokens, new.cache_creation_input_tokens),
            cache_read_input_tokens=max(existing.cache_read_input_tokens, new.cache_read_input_tokens),
        )

    async def workflow_events(self):
        """Generator yielding workflow SSE events (using proper SSE parsing)."""
        session = await self._get_session()
        try:
            async with session.get(f"{self.base_url}/workflow/events") as resp:
                async for payload in _iter_sse_events(resp):
                    try:
                        yield json.loads(payload)
                    except json.JSONDecodeError:
                        continue
        except (aiohttp.ClientError, asyncio.CancelledError):
            pass

    async def workflow_action(self, action: str) -> Dict[str, Any]:
        session = await self._get_session()
        endpoint = f"{self.base_url}/workflow/{action}"
        async with session.post(endpoint) as resp:
            return await resp.json()

    async def fetch_providers(self) -> List[Dict[str, Any]]:
        try:
            session = await self._get_session()
            async with session.get(f"{self.base_url}/providers") as resp:
                return await resp.json()
        except Exception:
            return []
