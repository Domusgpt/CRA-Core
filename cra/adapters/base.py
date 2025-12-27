"""Base adapter interface for CRA.

Adapters translate CRA resolutions to platform-specific formats.
"""

from abc import ABC, abstractmethod
from typing import Any

from cra.core.atlas import LoadedAtlas
from cra.core.carp import AllowedAction, ContextBlock, Resolution


class AdapterOutput(ABC):
    """Base class for adapter output."""

    @abstractmethod
    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary format."""
        pass

    @abstractmethod
    def to_string(self) -> str:
        """Convert to string format."""
        pass


class BaseAdapter(ABC):
    """Base class for platform adapters.

    Adapters are responsible for translating CRA resolutions
    into platform-specific formats (tool schemas, prompts, etc.).
    """

    @property
    @abstractmethod
    def platform_name(self) -> str:
        """Get the platform name."""
        pass

    @abstractmethod
    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> AdapterOutput:
        """Emit tool definitions for the platform.

        Args:
            actions: List of allowed actions from resolution
            atlas: Optional Atlas for additional context

        Returns:
            Platform-specific tool definitions
        """
        pass

    @abstractmethod
    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> AdapterOutput:
        """Emit context for the platform.

        Args:
            context_blocks: Context blocks from resolution
            resolution: The full resolution

        Returns:
            Platform-specific context format
        """
        pass

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> AdapterOutput:
        """Emit full platform-specific output.

        Combines tools and context into a single output.

        Args:
            resolution: The CARP resolution
            atlas: Optional Atlas for additional context

        Returns:
            Complete platform-specific output
        """
        tools_output = self.emit_tools(resolution.allowed_actions, atlas)
        context_output = self.emit_context(resolution.context_blocks, resolution)

        # Default implementation combines them
        return CombinedOutput(
            tools=tools_output.to_dict(),
            context=context_output.to_dict(),
        )


class CombinedOutput(AdapterOutput):
    """Combined tools and context output."""

    def __init__(self, tools: dict[str, Any], context: dict[str, Any]) -> None:
        self.tools = tools
        self.context = context

    def to_dict(self) -> dict[str, Any]:
        return {"tools": self.tools, "context": self.context}

    def to_string(self) -> str:
        import json
        return json.dumps(self.to_dict(), indent=2)


class JSONOutput(AdapterOutput):
    """JSON-based adapter output."""

    def __init__(self, data: dict[str, Any]) -> None:
        self.data = data

    def to_dict(self) -> dict[str, Any]:
        return self.data

    def to_string(self) -> str:
        import json
        return json.dumps(self.data, indent=2)


class MarkdownOutput(AdapterOutput):
    """Markdown-based adapter output."""

    def __init__(self, content: str, metadata: dict[str, Any] | None = None) -> None:
        self.content = content
        self.metadata = metadata or {}

    def to_dict(self) -> dict[str, Any]:
        return {"content": self.content, "metadata": self.metadata}

    def to_string(self) -> str:
        return self.content
