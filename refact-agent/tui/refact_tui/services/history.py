"""Chat history persistence — save/load conversations to the CraftifAI config dir."""
from __future__ import annotations

import json
import os
import platform
import time
from pathlib import Path
from typing import Optional, List, Dict, Any
from dataclasses import dataclass

from refact_tui.services.agent_client import Message


def _get_history_dir() -> Path:
    """Return the chat history storage directory, following CraftifAI config pattern."""
    if platform.system() == "Windows":
        base = os.environ.get("APPDATA", os.path.join(os.path.expanduser("~"), "AppData", "Roaming"))
        return Path(base) / "craftifai" / "tui_history"
    else:
        return Path.home() / ".config" / "craftifai" / "tui_history"


@dataclass
class ChatHistoryEntry:
    """One saved conversation."""
    chat_id: str
    title: str
    model: str
    created_at: float
    updated_at: float
    message_count: int
    file_path: str

    @property
    def display_title(self) -> str:
        """Short display title with timestamp."""
        age = time.time() - self.updated_at
        if age < 3600:
            ago = f"{int(age / 60)}m ago"
        elif age < 86400:
            ago = f"{int(age / 3600)}h ago"
        else:
            ago = f"{int(age / 86400)}d ago"
        title = self.title if len(self.title) <= 50 else self.title[:47] + "…"
        return f"{title}  [{ago}, {self.message_count} msgs]"


class ChatHistory:
    """Manages persisted chat conversations."""

    def __init__(self):
        self._dir = _get_history_dir()
        self._dir.mkdir(parents=True, exist_ok=True)

    def save(
        self,
        chat_id: str,
        messages: List[Message],
        model: str = "",
        title: Optional[str] = None,
    ) -> str:
        """Save a conversation. Returns the file path."""
        if not messages:
            return ""

        file_path = self._dir / f"{chat_id}.json"

        # Auto-generate title from first user message if not provided
        if not title:
            for m in messages:
                if m.role == "user" and isinstance(m.content, str) and m.content.strip():
                    title = m.content.strip()[:60]
                    break
            if not title:
                title = f"Chat {chat_id[:8]}"

        # Count non-system messages
        msg_count = sum(1 for m in messages if m.role not in ("system",))

        now = time.time()
        created_at = now
        # Preserve original created_at if file exists
        if file_path.exists():
            try:
                existing = json.loads(file_path.read_text(encoding="utf-8"))
                created_at = existing.get("created_at", now)
            except Exception:
                pass

        data = {
            "chat_id": chat_id,
            "title": title,
            "model": model,
            "created_at": created_at,
            "updated_at": now,
            "message_count": msg_count,
            "messages": [m.model_dump(exclude_none=True) for m in messages],
        }

        file_path.write_text(json.dumps(data, indent=2), encoding="utf-8")
        return str(file_path)

    def list_chats(self, limit: int = 30) -> List[ChatHistoryEntry]:
        """List recent chat history entries, newest first."""
        entries = []
        for f in self._dir.glob("*.json"):
            try:
                data = json.loads(f.read_text(encoding="utf-8"))
                entries.append(ChatHistoryEntry(
                    chat_id=data.get("chat_id", f.stem),
                    title=data.get("title", "Untitled"),
                    model=data.get("model", ""),
                    created_at=data.get("created_at", 0),
                    updated_at=data.get("updated_at", 0),
                    message_count=data.get("message_count", 0),
                    file_path=str(f),
                ))
            except Exception:
                continue

        entries.sort(key=lambda e: e.updated_at, reverse=True)
        return entries[:limit]

    def load(self, chat_id: str) -> Optional[Dict[str, Any]]:
        """Load a full conversation by chat_id."""
        file_path = self._dir / f"{chat_id}.json"
        if not file_path.exists():
            return None
        try:
            data = json.loads(file_path.read_text(encoding="utf-8"))
            return data
        except Exception:
            return None

    def load_messages(self, chat_id: str) -> List[Message]:
        """Load messages from a saved conversation."""
        data = self.load(chat_id)
        if not data or "messages" not in data:
            return []
        msgs = []
        for m in data["messages"]:
            try:
                msgs.append(Message(**m))
            except Exception:
                continue
        return msgs

    def delete(self, chat_id: str) -> bool:
        """Delete a saved conversation."""
        file_path = self._dir / f"{chat_id}.json"
        try:
            file_path.unlink(missing_ok=True)
            return True
        except Exception:
            return False
