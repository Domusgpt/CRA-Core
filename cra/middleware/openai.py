"""OpenAI-specific CRA middleware.

Integrates CRA governance with OpenAI's function calling.
"""

from typing import Any

from cra.middleware.base import CRAMiddleware


class OpenAIMiddleware(CRAMiddleware):
    """Middleware for OpenAI integration.

    Provides tools in OpenAI function calling format
    and handles tool call execution through CRA.

    Usage:
        from openai import OpenAI
        from cra.middleware import OpenAIMiddleware

        client = OpenAI()
        middleware = OpenAIMiddleware()

        # Get CRA-governed tools
        tools = middleware.get_tools(
            goal="Help user with data analysis",
            atlas_id="com.example.data-analytics",
        )

        # Use with OpenAI
        response = client.chat.completions.create(
            model="gpt-4",
            messages=[{"role": "user", "content": "Run a query..."}],
            tools=tools,
        )

        # Handle tool calls through CRA
        if response.choices[0].message.tool_calls:
            for tool_call in response.choices[0].message.tool_calls:
                result = middleware.handle_tool_call(tool_call)
    """

    def get_tools(
        self,
        goal: str,
        atlas_id: str | None = None,
        capability: str | None = None,
    ) -> list[dict[str, Any]]:
        """Get tools in OpenAI format.

        Args:
            goal: The agent's goal
            atlas_id: Optional Atlas ID
            capability: Optional capability filter

        Returns:
            List of OpenAI tool definitions
        """
        self.resolve(goal, atlas_id, capability)

        tools = []
        for action in self._resolution.allowed_actions:
            tool = {
                "type": "function",
                "function": {
                    "name": action.get("action_id", "").replace(".", "_"),
                    "description": action.get("description", ""),
                    "parameters": action.get("schema", {
                        "type": "object",
                        "properties": {},
                    }),
                },
            }
            tools.append(tool)

        return tools

    def handle_tool_call(
        self,
        tool_call: Any,
    ) -> dict[str, Any]:
        """Handle an OpenAI tool call through CRA.

        Args:
            tool_call: OpenAI tool call object

        Returns:
            Execution result
        """
        import json

        # Get action ID (convert back from function name)
        action_id = tool_call.function.name.replace("_", ".")

        # Parse arguments
        try:
            parameters = json.loads(tool_call.function.arguments)
        except json.JSONDecodeError:
            parameters = {}

        # Execute through CRA
        return self.execute(action_id, parameters)

    def create_tool_message(
        self,
        tool_call_id: str,
        result: dict[str, Any],
    ) -> dict[str, Any]:
        """Create an OpenAI tool message from execution result.

        Args:
            tool_call_id: The tool call ID
            result: Execution result from CRA

        Returns:
            OpenAI tool message
        """
        import json

        content = json.dumps({
            "success": result.get("status") == "completed",
            "result": result.get("result"),
            "error": result.get("error"),
            "trace_id": str(self.get_trace_id()) if self.get_trace_id() else None,
        })

        return {
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": content,
        }

    def get_system_message(
        self,
        atlas_id: str | None = None,
    ) -> str:
        """Get a system message with CRA governance info.

        Args:
            atlas_id: Optional Atlas ID for context

        Returns:
            System message string
        """
        message = """You are operating under CRA (Context Registry Agents) governance.

Important rules:
1. Only use the provided tools for actions - do not simulate or bypass them
2. Respect any denials or constraints in the context
3. Actions requiring approval will be flagged - do not proceed without approval
4. Reference trace_id when confirming action completion

"""

        if self._resolution:
            if self._resolution.denylist:
                message += "\nDenied patterns (DO NOT attempt):\n"
                for rule in self._resolution.denylist:
                    message += f"  - {rule.get('pattern')}: {rule.get('reason')}\n"

            message += f"\nTrace ID: {self.get_trace_id()}\n"
            message += f"Resolution confidence: {self._resolution.confidence:.0%}\n"

        return message
