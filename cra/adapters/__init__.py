"""Platform adapters for CRA.

Adapters translate CRA resolutions to platform-specific formats:
- OpenAI: Tool/function schemas
- Anthropic/Claude: SKILL.md format
- Google ADK: AgentTool stubs
- MCP: Server descriptors
"""

from cra.adapters.base import (
    AdapterOutput,
    BaseAdapter,
    CombinedOutput,
    JSONOutput,
    MarkdownOutput,
)
from cra.adapters.openai import OpenAIAdapter, create_openai_adapter
from cra.adapters.anthropic import AnthropicAdapter, create_anthropic_adapter
from cra.adapters.google_adk import GoogleADKAdapter, create_google_adk_adapter
from cra.adapters.mcp import MCPAdapter, create_mcp_adapter

__all__ = [
    # Base types
    "AdapterOutput",
    "BaseAdapter",
    "CombinedOutput",
    "JSONOutput",
    "MarkdownOutput",
    # Platform adapters
    "OpenAIAdapter",
    "create_openai_adapter",
    "AnthropicAdapter",
    "create_anthropic_adapter",
    "GoogleADKAdapter",
    "create_google_adk_adapter",
    "MCPAdapter",
    "create_mcp_adapter",
]


def get_adapter(platform: str) -> BaseAdapter:
    """Get an adapter for a platform.

    Args:
        platform: Platform name (openai, anthropic, google_adk, mcp)

    Returns:
        The adapter instance

    Raises:
        ValueError: If platform not supported
    """
    adapters = {
        "openai": create_openai_adapter,
        "anthropic": create_anthropic_adapter,
        "google_adk": create_google_adk_adapter,
        "mcp": create_mcp_adapter,
    }

    if platform not in adapters:
        raise ValueError(
            f"Unknown platform: {platform}. "
            f"Supported: {', '.join(adapters.keys())}"
        )

    return adapters[platform]()
