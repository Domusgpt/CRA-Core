"""OpenAI adapter for CRA.

Translates CRA resolutions to OpenAI function calling format.
"""

from typing import Any

from cra.adapters.base import AdapterOutput, BaseAdapter, JSONOutput, MarkdownOutput
from cra.core.atlas import LoadedAtlas
from cra.core.carp import ActionKind, AllowedAction, ContextBlock, Resolution


class OpenAIToolsOutput(AdapterOutput):
    """OpenAI tools output format."""

    def __init__(self, tools: list[dict[str, Any]]) -> None:
        self.tools = tools

    def to_dict(self) -> dict[str, Any]:
        return {"tools": self.tools}

    def to_string(self) -> str:
        import json
        return json.dumps(self.to_dict(), indent=2)


class OpenAIAdapter(BaseAdapter):
    """Adapter for OpenAI function calling.

    Generates tool schemas compatible with OpenAI's function calling API.
    """

    @property
    def platform_name(self) -> str:
        return "openai"

    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> OpenAIToolsOutput:
        """Emit OpenAI function calling tools.

        Generates the tools array for OpenAI chat completions.

        Args:
            actions: List of allowed actions
            atlas: Optional Atlas for additional context

        Returns:
            OpenAI tools format
        """
        tools = []

        for action in actions:
            # Convert action_id to valid function name (replace dots/dashes)
            function_name = action.action_id.replace(".", "_").replace("-", "_")

            tool = {
                "type": "function",
                "function": {
                    "name": function_name,
                    "description": self._build_description(action),
                    "parameters": self._convert_schema(action.schema),
                },
            }

            # Add strict mode if schema is well-defined
            if action.schema.get("properties"):
                tool["function"]["strict"] = True

            tools.append(tool)

        return OpenAIToolsOutput(tools)

    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> MarkdownOutput:
        """Emit context as a system message.

        Generates markdown content suitable for a system message.

        Args:
            context_blocks: Context blocks from resolution
            resolution: The full resolution

        Returns:
            Markdown content for system message
        """
        sections = []

        # Header
        sections.append("# CRA Context")
        sections.append("")
        sections.append(f"Resolution ID: `{resolution.resolution_id}`")
        sections.append(f"Confidence: {resolution.confidence:.0%}")
        sections.append("")

        # Context blocks
        for block in context_blocks:
            sections.append(f"## {block.purpose}")
            sections.append("")
            if isinstance(block.content, str):
                sections.append(block.content)
            else:
                import json
                sections.append(f"```json\n{json.dumps(block.content, indent=2)}\n```")
            sections.append("")
            sections.append(f"*TTL: {block.ttl_seconds}s*")
            sections.append("")

        # Deny rules
        if resolution.denylist:
            sections.append("## Deny Rules")
            sections.append("")
            sections.append("Do NOT attempt the following:")
            sections.append("")
            for rule in resolution.denylist:
                sections.append(f"- `{rule.pattern}`: {rule.reason}")
            sections.append("")

        # Allowed actions summary
        if resolution.allowed_actions:
            sections.append("## Available Actions")
            sections.append("")
            for action in resolution.allowed_actions:
                approval_note = " (requires approval)" if action.requires_approval else ""
                sections.append(f"- `{action.action_id}`: {action.description}{approval_note}")
            sections.append("")

        return MarkdownOutput("\n".join(sections))

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> JSONOutput:
        """Emit complete OpenAI configuration.

        Returns:
            Combined tools and system message
        """
        tools_output = self.emit_tools(resolution.allowed_actions, atlas)
        context_output = self.emit_context(resolution.context_blocks, resolution)

        return JSONOutput({
            "tools": tools_output.tools,
            "system_message": context_output.content,
            "metadata": {
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
                "platform": self.platform_name,
            },
        })

    def _build_description(self, action: AllowedAction) -> str:
        """Build a description for an action."""
        desc = action.description

        # Add constraint notes
        if action.requires_approval:
            desc += " [Requires approval before execution]"

        if action.constraints:
            constraint_notes = []
            for c in action.constraints:
                if c.type.value == "rate_limit":
                    constraint_notes.append(f"Rate limited")
                elif c.type.value == "scope":
                    constraint_notes.append(f"Requires scope: {c.value}")
            if constraint_notes:
                desc += f" [{', '.join(constraint_notes)}]"

        return desc

    def _convert_schema(self, schema: dict[str, Any]) -> dict[str, Any]:
        """Convert CRA schema to OpenAI JSON Schema format.

        OpenAI uses standard JSON Schema with some restrictions.
        """
        if not schema:
            return {
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": False,
            }

        # Ensure required fields for OpenAI strict mode
        result = dict(schema)

        if "type" not in result:
            result["type"] = "object"

        if result["type"] == "object":
            if "properties" not in result:
                result["properties"] = {}
            if "required" not in result:
                result["required"] = []
            if "additionalProperties" not in result:
                result["additionalProperties"] = False

        return result


def create_openai_adapter() -> OpenAIAdapter:
    """Create an OpenAI adapter instance."""
    return OpenAIAdapter()
