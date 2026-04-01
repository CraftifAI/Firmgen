"""
PDF parser — extracts text from PDF files using PyMuPDF (fitz).
"""

import io
import logging
from .base import BaseParser

logger = logging.getLogger(__name__)


class PdfParser(BaseParser):
    """Extract text from PDF files page-by-page using PyMuPDF."""

    supported_mime_types = ["application/pdf"]

    def parse(self, data: bytes, filename: str = "") -> dict:
        try:
            import fitz  # PyMuPDF
        except ImportError:
            raise RuntimeError(
                "PyMuPDF is not installed. Run: pip install PyMuPDF"
            )

        pages_text: list[str] = []
        metadata: dict = {}

        try:
            doc = fitz.open(stream=data, filetype="pdf")
            metadata = {
                "page_count": doc.page_count,
                "author": doc.metadata.get("author", ""),
                "title": doc.metadata.get("title", ""),
                "subject": doc.metadata.get("subject", ""),
                "creator": doc.metadata.get("creator", ""),
                "producer": doc.metadata.get("producer", ""),
            }

            for page_num in range(doc.page_count):
                page = doc[page_num]
                page_text = page.get_text("text")  # type: ignore[attr-defined]
                pages_text.append(page_text)

            doc.close()
        except Exception as exc:
            logger.error(f"PDF parsing failed for '{filename}': {exc}")
            raise ValueError(f"Failed to parse PDF: {exc}") from exc

        full_text = "\n\n".join(pages_text)
        return {
            "text": full_text,
            "page_count": metadata.get("page_count", 0),
            "metadata": metadata,
        }
