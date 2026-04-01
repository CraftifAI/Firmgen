"""
Base parser interface for all file parsers.
"""

from abc import ABC, abstractmethod


class BaseParser(ABC):
    """Abstract base class for all file parsers.

    Each concrete parser must implement `parse(data: bytes) -> dict` and
    expose a `supported_mime_types` class attribute listing what MIME types
    it handles.
    """

    supported_mime_types: list[str] = []

    @abstractmethod
    def parse(self, data: bytes, filename: str = "") -> dict:
        """Parse raw file bytes and return a structured dict.

        Args:
            data: Raw file bytes.
            filename: Original filename (used for metadata / logging).

        Returns:
            A dict with at minimum a ``text`` key containing the extracted
            plain-text content plus any format-specific metadata keys.
        """

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} mime_types={self.supported_mime_types}>"
