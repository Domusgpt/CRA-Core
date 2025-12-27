"""Google ADK (Agent Development Kit) adapter for CRA.

Translates CRA resolutions to Google ADK AgentTool format.
"""

from typing import Any

from cra.adapters.base import AdapterOutput, BaseAdapter, JSONOutput, MarkdownOutput
from cra.core.atlas import LoadedAtlas
from cra.core.carp import AllowedAction, ContextBlock, Resolution


class ADKToolsOutput(AdapterOutput):
    """Google ADK tools output format."""

    def __init__(self, tools: list[dict[str, Any]]) -> None:
        self.tools = tools

    def to_dict(self) -> dict[str, Any]:
        return {"agent_tools": self.tools}

    def to_string(self) -> str:
        import json
        return json.dumps(self.to_dict(), indent=2)


class GoogleADKAdapter(BaseAdapter):
    """Adapter for Google Agent Development Kit.

    Generates AgentTool definitions compatible with Google's ADK.
    """

    @property
    def platform_name(self) -> str:
        return "google_adk"

    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> ADKToolsOutput:
        """Emit Google ADK AgentTool definitions.

        Args:
            actions: List of allowed actions
            atlas: Optional Atlas for additional context

        Returns:
            ADK AgentTool format
        """
        tools = []

        for action in actions:
            tool = {
                "name": action.action_id,
                "description": action.description,
                "parameters": self._convert_to_adk_schema(action.schema),
                "orchestration_hints": {
                    "requires_approval": action.requires_approval,
                    "timeout_ms": action.timeout_ms,
                    "kind": action.kind.value,
                },
                "constraints": [
                    {"type": c.type.value, "value": c.value}
                    for c in action.constraints
                ],
            }

            # Add CRA-specific metadata
            tool["cra_metadata"] = {
                "action_id": action.action_id,
                "adapter": action.adapter,
            }

            tools.append(tool)

        return ADKToolsOutput(tools)

    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> JSONOutput:
        """Emit context in ADK-compatible format.

        Args:
            context_blocks: Context blocks from resolution
            resolution: The full resolution

        Returns:
            JSON context for ADK
        """
        context = {
            "resolution_id": str(resolution.resolution_id),
            "confidence": resolution.confidence,
            "context_blocks": [],
            "deny_rules": [],
            "next_steps": [],
        }

        for block in context_blocks:
            context["context_blocks"].append({
                "block_id": block.block_id,
                "purpose": block.purpose,
                "ttl_seconds": block.ttl_seconds,
                "content_type": block.content_type.value,
                "content": block.content,
            })

        for rule in resolution.denylist:
            context["deny_rules"].append({
                "pattern": rule.pattern,
                "reason": rule.reason,
            })

        for step in resolution.next_steps:
            context["next_steps"].append({
                "step": step.step,
                "expected_artifacts": step.expected_artifacts,
            })

        return JSONOutput(context)

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> JSONOutput:
        """Emit complete ADK configuration.

        Returns:
            Combined AgentTools and context
        """
        tools_output = self.emit_tools(resolution.allowed_actions, atlas)
        context_output = self.emit_context(resolution.context_blocks, resolution)

        return JSONOutput({
            "agent_tools": tools_output.tools,
            "context": context_output.to_dict(),
            "agent_config": {
                "name": f"cra_agent_{resolution.resolution_id}",
                "description": "CRA-governed agent",
                "model": "gemini-pro",  # Default, can be overridden
                "instruction": self._generate_instruction(resolution),
            },
            "metadata": {
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
                "platform": self.platform_name,
            },
        })

    def _convert_to_adk_schema(self, schema: dict[str, Any]) -> dict[str, Any]:
        """Convert CRA schema to ADK parameter format.

        ADK uses a similar format to JSON Schema but with some differences.
        """
        if not schema:
            return {"type": "object", "properties": {}}

        # ADK expects parameters in a specific format
        result = {
            "type": schema.get("type", "object"),
        }

        if "properties" in schema:
            result["properties"] = {}
            for name, prop in schema["properties"].items():
                result["properties"][name] = {
                    "type": prop.get("type", "string"),
                    "description": prop.get("description", ""),
                }
                if "enum" in prop:
                    result["properties"][name]["enum"] = prop["enum"]

        if "required" in schema:
            result["required"] = schema["required"]

        return result

    def _generate_instruction(self, resolution: Resolution) -> str:
        """Generate agent instruction text."""
        lines = [
            "You are operating under CRA (Context Registry Agent) governance.",
            "",
            f"Resolution ID: {resolution.resolution_id}",
            f"Confidence: {resolution.confidence:.0%}",
            "",
            "Guidelines:",
            "1. Use only the provided AgentTools for actions",
            "2. Respect deny rules - do not attempt blocked patterns",
            "3. Actions marked as requiring approval need explicit approval",
            "4. TRACE output is authoritative for execution confirmation",
            "",
        ]

        if resolution.denylist:
            lines.append("Deny Patterns (DO NOT attempt):")
            for rule in resolution.denylist:
                lines.append(f"  - {rule.pattern}: {rule.reason}")
            lines.append("")

        return "\n".join(lines)


def create_google_adk_adapter() -> GoogleADKAdapter:
    """Create a Google ADK adapter instance."""
    return GoogleADKAdapter()
