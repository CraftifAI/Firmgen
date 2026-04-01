"""
Plain-text file parser.
"""

from .base import BaseParser


class TxtParser(BaseParser):
    """Parse plain-text files (.txt, .md, .csv, etc.) via UTF-8 with Latin-1 fallback."""

    supported_mime_types = [
        "text/plain",
        "text/markdown",
        "text/csv",
        "text/x-python",
        "text/x-java-source",
        "application/json",
    ]

    def parse(self, data: bytes, filename: str = "") -> dict:
        # Try UTF-8 first; fall back to Latin-1 to avoid crashes on legacy files.
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            text = data.decode("latin-1")

        lines = text.splitlines()
        return {
            "text": text,
            "line_count": len(lines),
            "char_count": len(text),
        }
