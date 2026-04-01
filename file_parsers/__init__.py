"""
File parser registry — maps MIME types and extensions to parser classes,
and exposes a single ``parse_file()`` function for the upload endpoint.
"""

import mimetypes
import logging
from pathlib import Path
from typing import Optional

from .base import BaseParser
from .txt_parser import TxtParser
from .pdf_parser import PdfParser
from .docx_parser import DocxParser
from .pptx_parser import PptxParser
from .xlsx_parser import XlsxParser

logger = logging.getLogger(__name__)

# ── Registry ──────────────────────────────────────────────────────────────────

_PARSERS: list[BaseParser] = [
    PdfParser(),
    DocxParser(),
    PptxParser(),
    XlsxParser(),
    TxtParser(),  # Keep last as a catch-all for plain text variants
]

# Build a flat MIME → parser lookup for O(1) dispatch
_MIME_TO_PARSER: dict[str, BaseParser] = {}
for _parser in _PARSERS:
    for _mime in _parser.supported_mime_types:
        _MIME_TO_PARSER[_mime] = _parser

# Extension-to-MIME fallback for when the client sends a generic content-type
_EXT_TO_MIME: dict[str, str] = {
    ".pdf":  "application/pdf",
    ".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ".doc":  "application/msword",
    ".pptx": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ".ppt":  "application/vnd.ms-powerpoint",
    ".xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ".xls":  "application/vnd.ms-excel",
    ".txt":  "text/plain",
    ".md":   "text/markdown",
    ".csv":  "text/csv",
    ".json": "application/json",
    ".py":   "text/x-python",
    ".java": "text/x-java-source",
}

SUPPORTED_EXTENSIONS = list(_EXT_TO_MIME.keys())
SUPPORTED_MIME_TYPES = list(_MIME_TO_PARSER.keys())


def resolve_mime_type(filename: str, content_type: Optional[str] = None) -> str:
    """Return the effective MIME type for a file.

    Priority:
    1. Extension-based lookup (most reliable when client sends 'application/octet-stream')
    2. Content-Type header value from the client
    3. Python's mimetypes library as a last resort
    """
    ext = Path(filename).suffix.lower()
    if ext in _EXT_TO_MIME:
        return _EXT_TO_MIME[ext]

    if content_type and content_type not in ("application/octet-stream", ""):
        return content_type

    guessed, _ = mimetypes.guess_type(filename)
    return guessed or "application/octet-stream"


def get_parser(mime_type: str) -> Optional[BaseParser]:
    """Return the parser for a MIME type, or None if unsupported."""
    return _MIME_TO_PARSER.get(mime_type)


def parse_file(filename: str, data: bytes, content_type: Optional[str] = None) -> dict:
    """Parse uploaded file bytes and return structured text + metadata.

    Args:
        filename: Original filename (used for extension detection and logging).
        data: Raw file bytes.
        content_type: Optional MIME type provided by the client.

    Returns:
        A dict with keys:
        - ``text``        — Extracted plain-text content
        - ``mime_type``  — Resolved MIME type
        - any extras from the specific parser (e.g. ``page_count``)

    Raises:
        ValueError: If the file type is not supported or parsing fails.
    """
    mime_type = resolve_mime_type(filename, content_type)
    logger.info(f"Parsing '{filename}' as {mime_type} ({len(data)} bytes)")

    parser = get_parser(mime_type)
    if parser is None:
        supported = ", ".join(sorted(SUPPORTED_EXTENSIONS))
        raise ValueError(
            f"Unsupported file type '{mime_type}'. "
            f"Supported extensions: {supported}"
        )

    result = parser.parse(data, filename=filename)
    result["mime_type"] = mime_type
    return result
