"""LangChain-specific CRA middleware.

Integrates CRA governance with LangChain tools and agents.
"""

from typing import Any, Callable

from cra.middleware.base import CRAMiddleware


class LangChainMiddleware(CRAMiddleware):
    """Middleware for LangChain integration.

    Provides tools as LangChain Tool objects with CRA governance.

    Usage:
        from langchain_openai import ChatOpenAI
        from langchain.agents import create_openai_functions_agent, AgentExecutor
        from cra.middleware import LangChainMiddleware

        middleware = LangChainMiddleware()

        # Get CRA-governed tools
        tools = middleware.get_tools(
            goal="Help user with data analysis",
            atlas_id="com.example.data-analytics",
        )

        # Create agent with tools
        llm = ChatOpenAI(model="gpt-4")
        agent = create_openai_functions_agent(llm, tools, prompt)
        executor = AgentExecutor(agent=agent, tools=tools)
    """

    def get_tools(
        self,
        goal: str,
        atlas_id: str | None = None,
        capability: str | None = None,
    ) -> list[Any]:
        """Get tools as LangChain Tool objects.

        Args:
            goal: The agent's goal
            atlas_id: Optional Atlas ID
            capability: Optional capability filter

        Returns:
            List of LangChain Tool objects
        """
        try:
            from langchain.tools import StructuredTool
            from pydantic import create_model, Field
        except ImportError:
            raise RuntimeError(
                "LangChain is required for this middleware. "
                "Install with: pip install langchain"
            )

        self.resolve(goal, atlas_id, capability)

        tools = []
        for action in self._resolution.allowed_actions:
            action_id = action.get("action_id", "")
            description = action.get("description", f"Execute {action_id}")
            schema = action.get("schema", {"type": "object", "properties": {}})

            # Create the tool function
            tool_func = self._create_tool_function(action_id)

            # Create input model from schema
            input_model = self._schema_to_pydantic(action_id, schema)

            tool = StructuredTool.from_function(
                func=tool_func,
                name=action_id.replace(".", "_"),
                description=description,
                args_schema=input_model,
            )
            tools.append(tool)

        return tools

    def _create_tool_function(self, action_id: str) -> Callable[..., str]:
        """Create a tool function for an action.

        Args:
            action_id: The action ID

        Returns:
            Callable that executes the action
        """
        import json

        def tool_func(**kwargs: Any) -> str:
            result = self.execute(action_id, kwargs)
            return json.dumps({
                "success": result.get("status") == "completed",
                "result": result.get("result"),
                "error": result.get("error"),
                "trace_id": str(self.get_trace_id()) if self.get_trace_id() else None,
            })

        return tool_func

    def _schema_to_pydantic(
        self,
        name: str,
        schema: dict[str, Any],
    ) -> Any:
        """Convert JSON schema to Pydantic model.

        Args:
            name: Model name
            schema: JSON schema

        Returns:
            Pydantic model class
        """
        from pydantic import create_model, Field

        properties = schema.get("properties", {})
        required = set(schema.get("required", []))

        fields = {}
        for prop_name, prop_schema in properties.items():
            prop_type = self._json_type_to_python(prop_schema.get("type", "string"))
            description = prop_schema.get("description", "")

            if prop_name in required:
                fields[prop_name] = (prop_type, Field(description=description))
            else:
                fields[prop_name] = (
                    prop_type | None,
                    Field(default=None, description=description),
                )

        model_name = name.replace(".", "_").title() + "Input"
        return create_model(model_name, **fields)

    def _json_type_to_python(self, json_type: str) -> type:
        """Convert JSON schema type to Python type."""
        type_map = {
            "string": str,
            "integer": int,
            "number": float,
            "boolean": bool,
            "array": list,
            "object": dict,
        }
        return type_map.get(json_type, str)

    def get_runnable(
        self,
        goal: str,
        atlas_id: str | None = None,
        capability: str | None = None,
        model: str = "gpt-4",
    ) -> Any:
        """Get a LangChain Runnable with CRA tools.

        Args:
            goal: The agent's goal
            atlas_id: Optional Atlas ID
            capability: Optional capability filter
            model: Model to use

        Returns:
            LangChain Runnable
        """
        try:
            from langchain_openai import ChatOpenAI
            from langchain.agents import create_openai_functions_agent, AgentExecutor
            from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder
        except ImportError:
            raise RuntimeError(
                "LangChain packages required. Install with: "
                "pip install langchain langchain-openai"
            )

        tools = self.get_tools(goal, atlas_id, capability)

        llm = ChatOpenAI(model=model)

        prompt = ChatPromptTemplate.from_messages([
            ("system", self._get_system_prompt()),
            MessagesPlaceholder(variable_name="chat_history", optional=True),
            ("human", "{input}"),
            MessagesPlaceholder(variable_name="agent_scratchpad"),
        ])

        agent = create_openai_functions_agent(llm, tools, prompt)

        return AgentExecutor(
            agent=agent,
            tools=tools,
            verbose=True,
            handle_parsing_errors=True,
        )

    def _get_system_prompt(self) -> str:
        """Get the system prompt for the agent."""
        prompt = """You are an AI assistant operating under CRA governance.

All your actions are validated against policy and logged to an immutable TRACE record.

Rules:
1. Only use the provided tools for actions
2. Respect any denials in the context
3. Actions requiring approval need explicit user confirmation
4. Reference trace_id when confirming actions
"""

        if self._resolution and self._resolution.denylist:
            prompt += "\n\nDenied patterns (DO NOT attempt):\n"
            for rule in self._resolution.denylist:
                prompt += f"- {rule.get('pattern')}: {rule.get('reason')}\n"

        return prompt
