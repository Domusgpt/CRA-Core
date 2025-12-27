"""Atlas service for CRA runtime.

Manages Atlas loading, caching, and resolution integration.
"""

from pathlib import Path
from typing import Any

from cra.adapters import get_adapter
from cra.core.atlas import (
    AtlasLoader,
    AtlasRegistry,
    LoadedAtlas,
    get_atlas_registry,
)
from cra.core.carp import (
    ActionKind,
    AllowedAction,
    ContextBlock,
    ContentType,
    DenyRule,
    Resolution,
)


class AtlasService:
    """Service for managing Atlases in the runtime.

    Provides:
    - Atlas loading and registration
    - Context extraction for resolutions
    - Adapter output generation
    """

    def __init__(self, registry: AtlasRegistry | None = None) -> None:
        """Initialize the Atlas service.

        Args:
            registry: Optional registry (uses global if not provided)
        """
        self._registry = registry or get_atlas_registry()
        self._loader = AtlasLoader()

    def load_atlas(self, path: Path) -> LoadedAtlas:
        """Load and register an Atlas.

        Args:
            path: Path to Atlas directory

        Returns:
            The loaded Atlas
        """
        atlas = self._registry.register(path)
        return atlas

    def get_atlas(self, atlas_id: str) -> LoadedAtlas | None:
        """Get an Atlas by ID.

        Args:
            atlas_id: The Atlas ID

        Returns:
            The Atlas or None
        """
        return self._registry.get(atlas_id)

    def list_atlases(self) -> list[LoadedAtlas]:
        """List all registered Atlases."""
        return self._registry.list_all()

    def unload_atlas(self, atlas_id: str) -> bool:
        """Unload an Atlas.

        Args:
            atlas_id: The Atlas ID

        Returns:
            True if unloaded, False if not found
        """
        return self._registry.unregister(atlas_id)

    def get_context_blocks(
        self,
        atlas: LoadedAtlas,
        capability: str | None = None,
    ) -> list[ContextBlock]:
        """Get context blocks from an Atlas.

        Args:
            atlas: The loaded Atlas
            capability: Optional capability to filter by

        Returns:
            List of context blocks
        """
        blocks = []

        for pack in atlas.context_packs:
            content_type = ContentType.MARKDOWN
            if pack.content_type == "application/json":
                content_type = ContentType.JSON
            elif pack.content_type == "text/plain":
                content_type = ContentType.PLAIN

            block = ContextBlock(
                block_id=f"{atlas.id}:{pack.path}",
                purpose=f"Atlas context: {pack.path}",
                ttl_seconds=3600,
                content_type=content_type,
                content=pack.content,
            )
            blocks.append(block)

        return blocks

    def get_allowed_actions(
        self,
        atlas: LoadedAtlas,
        capability: str | None = None,
    ) -> list[AllowedAction]:
        """Get allowed actions from an Atlas.

        Args:
            atlas: The loaded Atlas
            capability: Optional capability to filter by

        Returns:
            List of allowed actions
        """
        actions = []

        # Check for OpenAI adapter format (most common)
        openai_adapter = atlas.adapters.get("openai")
        if openai_adapter and "tools" in openai_adapter:
            for tool in openai_adapter["tools"]:
                func = tool.get("function", {})
                action = AllowedAction(
                    action_id=func.get("name", "unknown"),
                    kind=ActionKind.TOOL_CALL,
                    adapter="openai",
                    description=func.get("description", ""),
                    json_schema=func.get("parameters", {}),
                )
                actions.append(action)

        # Check for MCP adapter format
        mcp_adapter = atlas.adapters.get("mcp")
        if mcp_adapter and "tools" in mcp_adapter:
            for tool in mcp_adapter["tools"]:
                action = AllowedAction(
                    action_id=tool.get("name", "unknown"),
                    kind=ActionKind.MCP_CALL,
                    adapter="mcp",
                    description=tool.get("description", ""),
                    json_schema=tool.get("inputSchema", {}),
                )
                actions.append(action)

        return actions

    def get_deny_rules(self, atlas: LoadedAtlas) -> list[DenyRule]:
        """Get deny rules from an Atlas's policies.

        Args:
            atlas: The loaded Atlas

        Returns:
            List of deny rules
        """
        rules = []

        for policy in atlas.policies:
            for rule in policy.rules:
                if rule.get("effect") == "deny":
                    for action in rule.get("actions", []):
                        deny_rule = DenyRule(
                            pattern=action,
                            reason=rule.get("reason", f"Denied by policy {policy.policy_id}"),
                        )
                        rules.append(deny_rule)

        return rules

    def emit_for_platform(
        self,
        resolution: Resolution,
        platform: str,
        atlas: LoadedAtlas | None = None,
    ) -> dict[str, Any]:
        """Emit resolution in platform-specific format.

        Args:
            resolution: The CARP resolution
            platform: Target platform (openai, anthropic, google_adk, mcp)
            atlas: Optional Atlas for additional context

        Returns:
            Platform-specific output
        """
        adapter = get_adapter(platform)
        output = adapter.emit_full(resolution, atlas)
        return output.to_dict()

    def get_atlas_summary(self, atlas: LoadedAtlas) -> dict[str, Any]:
        """Get a summary of an Atlas.

        Args:
            atlas: The loaded Atlas

        Returns:
            Summary dict
        """
        return {
            "id": atlas.id,
            "version": atlas.version,
            "name": atlas.name,
            "description": atlas.manifest.description,
            "capabilities": atlas.manifest.capabilities,
            "context_packs": len(atlas.context_packs),
            "policies": len(atlas.policies),
            "adapters": list(atlas.adapters.keys()),
            "certification": {
                "carp_compliant": atlas.manifest.certification.carp_compliant,
                "trace_compliant": atlas.manifest.certification.trace_compliant,
            },
        }


# Global service instance
_atlas_service: AtlasService | None = None


def get_atlas_service() -> AtlasService:
    """Get the global Atlas service."""
    global _atlas_service
    if _atlas_service is None:
        _atlas_service = AtlasService()
    return _atlas_service
