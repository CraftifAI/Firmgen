"""HTTP + SSE client for communicating with the Refact Agent engine."""
from __future__ import annotations

import json
import asyncio
import uuid
import os
import random
import subprocess
from typing import Optional, List, Dict, Any, Callable, Literal, DefaultDict, Union
from collections import defaultdict
from dataclasses import dataclass, field

import aiohttp
from pydantic import BaseModel, ConfigDict


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

    async def fetch_links(
        self,
        chat_id: str,
        messages: List[Message],
        model: str,
        chat_mode: str = "AGENT",
    ) -> List[Dict[str, Any]]:
        """Call POST /v1/links to get follow-up action buttons for the current chat state.

        Returns a list of link dicts, each with at least 'link_text' and 'link_action'.
        Only 'follow-up' actions are useful as clickable chat buttons.
        """
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
        on_data: Optional[Callable[[Dict[str, Any], ChoiceDeltaCollector], None]] = None,
    ) -> List[Message]:
        """Send a chat request with streaming; calls on_data for every SSE chunk."""
        msgs_dicts = []
        for m in messages:
            d = {"role": m.role, "content": m.content}
            if m.role == "assistant":
                if m.tool_calls:
                    d["tool_calls"] = [tc.model_dump(exclude_none=True) for tc in m.tool_calls]
                if m.finish_reason:
                    d["finish_reason"] = m.finish_reason
            elif m.role == "tool":
                d["tool_call_id"] = m.tool_call_id
            msgs_dicts.append(d)

        post_me = {
            "model": model,
            "messages": msgs_dicts,
            "temperature": temperature,
            "stream": True,
            "max_tokens": max_tokens,
            "only_deterministic_messages": False,
            "checkpoints_enabled": True,
            "meta": {
                "chat_id": chat_id,
                "chat_mode": "AGENT",
            },
        }

        deterministic: List[Message] = []
        subchats: DefaultDict[str, List[Message]] = defaultdict(list)
        deltas = ChoiceDeltaCollector(1)
        have_usage: Optional[Usage] = None

        session = await self._get_session()
        async with session.post(f"{self.base_url}/chat", json=post_me) as resp:
            buffer = b""
            async for data, end_of_chunk in resp.content.iter_chunks():
                buffer += data
                if not end_of_chunk:
                    continue
                line_str = buffer.decode("utf-8").strip()
                buffer = b""
                if not line_str:
                    continue
                if not line_str.startswith("data: "):
                    continue
                line_str = line_str[6:]
                if line_str == "[DONE]":
                    break
                j = json.loads(line_str)

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
        """Generator yielding workflow SSE events."""
        session = await self._get_session()
        try:
            async with session.get(f"{self.base_url}/workflow/events") as resp:
                buffer = b""
                async for data, end_of_chunk in resp.content.iter_chunks():
                    buffer += data
                    if not end_of_chunk:
                        continue
                    line_str = buffer.decode("utf-8").strip()
                    buffer = b""
                    if not line_str or not line_str.startswith("data: "):
                        continue
                    payload = line_str[6:]
                    if payload == "[DONE]":
                        break
                    yield json.loads(payload)
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
