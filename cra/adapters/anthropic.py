"""Anthropic/Claude adapter for CRA.

Translates CRA resolutions to Claude tool use format and SKILL.md.
"""

from typing import Any

from cra.adapters.base import AdapterOutput, BaseAdapter, JSONOutput, MarkdownOutput
from cra.core.atlas import LoadedAtlas
from cra.core.carp import AllowedAction, ContextBlock, Resolution


class ClaudeToolsOutput(AdapterOutput):
    """Claude tools output format."""

    def __init__(self, tools: list[dict[str, Any]]) -> None:
        self.tools = tools

    def to_dict(self) -> dict[str, Any]:
        return {"tools": self.tools}

    def to_string(self) -> str:
        import json
        return json.dumps(self.to_dict(), indent=2)


class SkillMDOutput(AdapterOutput):
    """SKILL.md output format for Claude."""

    def __init__(self, content: str, metadata: dict[str, Any] | None = None) -> None:
        self.content = content
        self.metadata = metadata or {}

    def to_dict(self) -> dict[str, Any]:
        return {"skill_md": self.content, "metadata": self.metadata}

    def to_string(self) -> str:
        return self.content


class AnthropicAdapter(BaseAdapter):
    """Adapter for Anthropic Claude.

    Generates:
    - Tool definitions for Claude tool use
    - SKILL.md format for agent context
    """

    @property
    def platform_name(self) -> str:
        return "anthropic"

    def emit_tools(
        self,
        actions: list[AllowedAction],
        atlas: LoadedAtlas | None = None,
    ) -> ClaudeToolsOutput:
        """Emit Claude tool use definitions.

        Generates the tools array for Claude API.

        Args:
            actions: List of allowed actions
            atlas: Optional Atlas for additional context

        Returns:
            Claude tools format
        """
        tools = []

        for action in actions:
            # Claude uses snake_case for tool names
            tool_name = action.action_id.replace(".", "_").replace("-", "_")

            tool = {
                "name": tool_name,
                "description": self._build_description(action),
                "input_schema": self._convert_schema(action.schema),
            }

            tools.append(tool)

        return ClaudeToolsOutput(tools)

    def emit_context(
        self,
        context_blocks: list[ContextBlock],
        resolution: Resolution,
    ) -> SkillMDOutput:
        """Emit context as SKILL.md format.

        Generates a SKILL.md document that can be injected
        into Claude's context.

        Args:
            context_blocks: Context blocks from resolution
            resolution: The full resolution

        Returns:
            SKILL.md content
        """
        sections = []

        # Title
        sections.append("# CRA Skill Context")
        sections.append("")

        # Metadata
        sections.append("## Metadata")
        sections.append("")
        sections.append(f"- **Resolution ID:** `{resolution.resolution_id}`")
        sections.append(f"- **Confidence:** {resolution.confidence:.0%}")
        sections.append(f"- **Actions Available:** {len(resolution.allowed_actions)}")
        sections.append("")

        # Context
        sections.append("## Context")
        sections.append("")
        for block in context_blocks:
            sections.append(f"### {block.purpose}")
            sections.append("")
            if isinstance(block.content, str):
                sections.append(block.content)
            else:
                import json
                sections.append(f"```json\n{json.dumps(block.content, indent=2)}\n```")
            sections.append("")

        # Available Actions
        sections.append("## Available Actions")
        sections.append("")
        for action in resolution.allowed_actions:
            sections.append(f"### `{action.action_id}`")
            sections.append("")
            sections.append(action.description)
            sections.append("")

            if action.requires_approval:
                sections.append("> **Note:** This action requires approval before execution.")
                sections.append("")

            if action.schema.get("properties"):
                sections.append("**Parameters:**")
                sections.append("")
                for prop_name, prop_def in action.schema["properties"].items():
                    prop_type = prop_def.get("type", "any")
                    prop_desc = prop_def.get("description", "")
                    required = prop_name in action.schema.get("required", [])
                    req_mark = " *(required)*" if required else ""
                    sections.append(f"- `{prop_name}` ({prop_type}): {prop_desc}{req_mark}")
                sections.append("")

        # Constraints
        sections.append("## Constraints")
        sections.append("")
        if resolution.denylist:
            sections.append("### Deny Patterns")
            sections.append("")
            sections.append("Do NOT attempt any of the following:")
            sections.append("")
            for rule in resolution.denylist:
                sections.append(f"- `{rule.pattern}`: {rule.reason}")
            sections.append("")

        # Guidelines
        sections.append("## Guidelines")
        sections.append("")
        sections.append("1. Always use the available actions rather than simulating behavior")
        sections.append("2. Respect TTLs on context blocks")
        sections.append("3. If an action requires approval, request it before proceeding")
        sections.append("4. TRACE output is authoritative - do not claim actions completed without confirmation")
        sections.append("")

        return SkillMDOutput(
            "\n".join(sections),
            metadata={
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
            },
        )

    def emit_full(
        self,
        resolution: Resolution,
        atlas: LoadedAtlas | None = None,
    ) -> JSONOutput:
        """Emit complete Claude configuration.

        Returns:
            Combined tools and SKILL.md
        """
        tools_output = self.emit_tools(resolution.allowed_actions, atlas)
        skill_output = self.emit_context(resolution.context_blocks, resolution)

        return JSONOutput({
            "tools": tools_output.tools,
            "skill_md": skill_output.content,
            "system_prompt_addition": self._generate_system_addition(resolution),
            "metadata": {
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
                "platform": self.platform_name,
            },
        })

    def _build_description(self, action: AllowedAction) -> str:
        """Build a description for an action."""
        desc = action.description

        if action.requires_approval:
            desc += " (Requires approval - request before executing)"

        return desc

    def _convert_schema(self, schema: dict[str, Any]) -> dict[str, Any]:
        """Convert CRA schema to Claude input_schema format.

        Claude uses standard JSON Schema.
        """
        if not schema:
            return {"type": "object", "properties": {}}

        return dict(schema)

    def _generate_system_addition(self, resolution: Resolution) -> str:
        """Generate additional system prompt content."""
        lines = [
            "",
            "## CRA Integration",
            "",
            "You are operating under CRA (Context Registry Agent) governance.",
            f"Resolution ID: {resolution.resolution_id}",
            "",
            "Rules:",
            "- Use only the provided tools for actions",
            "- Do not simulate or pretend to execute actions",
            "- Request approval for actions marked as requiring it",
            "- TRACE output is the source of truth for what happened",
            "",
        ]
        return "\n".join(lines)


def create_anthropic_adapter() -> AnthropicAdapter:
    """Create an Anthropic adapter instance."""
    return AnthropicAdapter()
