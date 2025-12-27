"""MCP (Model Context Protocol) adapter for CRA.

Translates CRA resolutions to MCP server descriptor format.
"""

from typing import Any

from cra.adapters.base import AdapterOutput, BaseAdapter, JSONOutput
from cra.core.atlas import LoadedAtlas
from cra.core.carp import AllowedAction, ContextBlock, Resolution


class MCPServerOutput(AdapterOutput):
    """MCP server descriptor output format."""

    def __init__(self, descriptor: dict[str, Any]) -> None:
        self.descriptor = descriptor

    def to_dict(self) -> dict[str, Any]:
        return self.descriptor

    def to_string(self) -> str:
        import json
        return json.dumps(self.descriptor, indent=2)


class MCPAdapter(BaseAdapter):
    """Adapter for Model Context Protocol.

    Generates MCP server descriptors with:
    - Tools
    - Resources
    - Prompts
    """

    @property
    def platform_name(self) -> str:
        return "mcp"

    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> MCPServerOutput:
        """Emit MCP tools definitions.

        Args:
            actions: List of allowed actions
            atlas: Optional Atlas for additional context

        Returns:
            MCP tools format
        """
        tools = []

        for action in actions:
            tool = {
                "name": action.action_id,
                "description": action.description,
                "inputSchema": self._convert_to_mcp_schema(action.schema),
            }

            # Add annotations for CRA-specific behavior
            annotations = {}
            if action.requires_approval:
                annotations["requiresApproval"] = True
            if action.timeout_ms:
                annotations["timeoutMs"] = action.timeout_ms

            if annotations:
                tool["annotations"] = annotations

            tools.append(tool)

        return MCPServerOutput({"tools": tools})

    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> MCPServerOutput:
        """Emit context as MCP resources and prompts.

        Args:
            context_blocks: Context blocks from resolution
            resolution: The full resolution

        Returns:
            MCP resources and prompts
        """
        resources = []
        prompts = []

        # Convert context blocks to resources
        for block in context_blocks:
            resource = {
                "uri": f"cra://context/{block.block_id}",
                "name": block.purpose,
                "mimeType": block.content_type.value,
                "description": f"Context: {block.purpose}",
            }
            resources.append(resource)

        # Create a prompt for the resolution
        prompt = {
            "name": "cra_context",
            "description": "CRA resolution context",
            "arguments": [
                {
                    "name": "resolution_id",
                    "description": "The CRA resolution ID",
                    "required": False,
                }
            ],
        }
        prompts.append(prompt)

        return MCPServerOutput({
            "resources": resources,
            "prompts": prompts,
        })

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> MCPServerOutput:
        """Emit complete MCP server descriptor.

        Returns:
            Full MCP server descriptor
        """
        tools_output = self.emit_tools(resolution.allowed_actions, atlas)
        context_output = self.emit_context(resolution.context_blocks, resolution)

        # Build full server descriptor
        server_name = f"cra-{atlas.id if atlas else 'default'}"

        descriptor = {
            "name": server_name,
            "version": atlas.version if atlas else "1.0.0",
            "description": "CRA-governed MCP server",
            "protocol_version": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {},
            },
            "tools": tools_output.descriptor.get("tools", []),
            "resources": context_output.descriptor.get("resources", []),
            "prompts": context_output.descriptor.get("prompts", []),
            "cra": {
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
                "deny_rules": [
                    {"pattern": r.pattern, "reason": r.reason}
                    for r in resolution.denylist
                ],
            },
        }

        return MCPServerOutput(descriptor)

    def emit_resource_contents(
        self,
        context_blocks: list[ContextBlock],
    ) -> dict[str, Any]:
        """Emit resource contents for MCP read requests.

        Args:
            context_blocks: Context blocks to convert

        Returns:
            Map of URI to content
        """
        contents = {}

        for block in context_blocks:
            uri = f"cra://context/{block.block_id}"
            contents[uri] = {
                "uri": uri,
                "mimeType": block.content_type.value,
                "text": block.content if isinstance(block.content, str) else None,
                "blob": None,  # Binary content not supported yet
            }

        return contents

    def _convert_to_mcp_schema(self, schema: dict[str, Any]) -> dict[str, Any]:
        """Convert CRA schema to MCP inputSchema format.

        MCP uses JSON Schema.
        """
        if not schema:
            return {"type": "object", "properties": {}}

        # MCP uses standard JSON Schema
        result = dict(schema)

        # Ensure type is set
        if "type" not in result:
            result["type"] = "object"

        return result


class MCPToolHandler:
    """Handles MCP tool calls with CRA integration.

    This can be used to implement an MCP server that
    routes tool calls through CRA.
    """

    def __init__(self, adapter: MCPAdapter) -> None:
        self.adapter = adapter
        self._resolution: Resolution | None = None

    def set_resolution(self, resolution: Resolution) -> None:
        """Set the current resolution for tool handling."""
        self._resolution = resolution

    async def handle_tool_call(
        self,
        name: str,
        arguments: dict[str, Any],
    ) -> dict[str, Any]:
        """Handle an MCP tool call.

        Args:
            name: Tool name
            arguments: Tool arguments

        Returns:
            Tool result

        Raises:
            ValueError: If tool not allowed
        """
        if not self._resolution:
            raise ValueError("No resolution set")

        # Check if tool is allowed
        allowed = False
        for action in self._resolution.allowed_actions:
            if action.action_id == name:
                allowed = True
                break

        if not allowed:
            raise ValueError(f"Tool '{name}' not allowed in current resolution")

        # Check deny patterns
        for rule in self._resolution.denylist:
            # Simple pattern matching (would be more sophisticated in production)
            if rule.pattern.replace("*", "") in name:
                raise ValueError(f"Tool '{name}' matches deny pattern: {rule.reason}")

        # Return a placeholder - actual execution would go through ActionExecutor
        return {
            "status": "pending",
            "message": f"Tool call '{name}' queued for execution through CRA",
            "resolution_id": str(self._resolution.resolution_id),
        }


def create_mcp_adapter() -> MCPAdapter:
    """Create an MCP adapter instance."""
    return MCPAdapter()
