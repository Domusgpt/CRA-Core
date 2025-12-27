"""LangChain/LangGraph template generator.

Generates LangChain tools and agents with CRA governance.
"""

from typing import Any

from cra.core.atlas import LoadedAtlas
from cra.core.carp import Resolution
from cra.templates.base import GeneratedFile, GeneratedTemplate, TemplateGenerator


class LangChainGenerator(TemplateGenerator):
    """Generator for LangChain/LangGraph integration.

    Creates LangChain tools that are backed by CRA governance,
    ensuring all tool calls go through CARP resolution.
    """

    @property
    def framework_name(self) -> str:
        return "langchain"

    @property
    def framework_version(self) -> str:
        return "0.1.0"

    def generate(
        self,
        atlas: LoadedAtlas,
        resolution: Resolution | None = None,
        config: dict[str, Any] | None = None,
    ) -> GeneratedTemplate:
        """Generate LangChain integration files.

        Args:
            atlas: The Atlas to generate from
            resolution: Optional resolution with allowed actions
            config: Optional config with:
                - use_langgraph: Generate LangGraph agent (default: True)
                - model: Model to use (default: gpt-4)

        Returns:
            Generated LangChain template
        """
        config = config or {}
        use_langgraph = config.get("use_langgraph", True)

        files = []

        # Generate CRA tools module
        tools_code = self._generate_tools_module(atlas)
        files.append(GeneratedFile(
            path="cra_tools.py",
            content=tools_code,
            description="LangChain tools backed by CRA",
        ))

        # Generate agent module
        if use_langgraph:
            agent_code = self._generate_langgraph_agent(atlas)
            files.append(GeneratedFile(
                path="cra_agent.py",
                content=agent_code,
                description="LangGraph agent with CRA governance",
            ))
        else:
            agent_code = self._generate_langchain_agent(atlas)
            files.append(GeneratedFile(
                path="cra_agent.py",
                content=agent_code,
                description="LangChain agent with CRA governance",
            ))

        # Generate main example
        main_code = self._generate_main(atlas)
        files.append(GeneratedFile(
            path="main.py",
            content=main_code,
            executable=True,
            description="Example usage script",
        ))

        # Generate requirements
        requirements = self._generate_requirements_content(use_langgraph)
        files.append(GeneratedFile(
            path="requirements.txt",
            content=requirements,
            description="Python dependencies",
        ))

        return GeneratedTemplate(
            framework=self.framework_name,
            files=files,
            instructions=self._generate_setup_instructions(atlas, use_langgraph),
            dependencies=requirements.split("\n"),
        )

    def _generate_tools_module(self, atlas: LoadedAtlas) -> str:
        """Generate LangChain tools backed by CRA."""
        capabilities = atlas.manifest.capabilities

        tool_definitions = ""
        for cap in capabilities:
            tool_name = cap.replace(".", "_")
            tool_definitions += f'''
class {tool_name.title().replace("_", "")}Tool(BaseTool):
    """Tool for {cap} capability."""

    name: str = "{tool_name}"
    description: str = "Execute the {cap} action through CRA governance"
    cra_client: "CRAClient"

    def _run(self, **kwargs: Any) -> str:
        """Execute the tool synchronously."""
        result = self.cra_client.execute_action("{cap}", kwargs)
        return json.dumps(result)

    async def _arun(self, **kwargs: Any) -> str:
        """Execute the tool asynchronously."""
        result = await self.cra_client.execute_action_async("{cap}", kwargs)
        return json.dumps(result)

'''

        return f'''"""CRA-backed LangChain tools for {atlas.manifest.name}.

These tools integrate with CRA governance to ensure all actions
are validated, logged, and auditable.
"""

import json
import os
from typing import Any, Optional

import httpx
from langchain.tools import BaseTool
from pydantic import Field


class CRAClient:
    """Client for CRA runtime communication."""

    def __init__(
        self,
        runtime_url: str | None = None,
        atlas_id: str = "{atlas.manifest.id}",
    ):
        self.runtime_url = runtime_url or os.getenv(
            "CRA_RUNTIME_URL", "http://localhost:8420"
        )
        self.atlas_id = atlas_id
        self.session_id: str | None = None
        self.trace_id: str | None = None
        self._client = httpx.Client(timeout=30.0)
        self._async_client: httpx.AsyncClient | None = None

    def create_session(self, principal_id: str = "langchain-agent") -> dict[str, Any]:
        """Create a CRA session."""
        response = self._client.post(
            f"{{self.runtime_url}}/v1/sessions",
            json={{
                "principal": {{"type": "agent", "id": principal_id}},
                "scopes": {capabilities},
            }},
        )
        response.raise_for_status()
        data = response.json()
        self.session_id = data["session_id"]
        self.trace_id = data["trace_id"]
        return data

    def resolve(self, goal: str, capability: str) -> dict[str, Any]:
        """Resolve context and permissions for a goal."""
        if not self.session_id:
            self.create_session()

        response = self._client.post(
            f"{{self.runtime_url}}/v1/carp/resolve",
            json={{
                "session_id": self.session_id,
                "goal": goal,
                "atlas_id": self.atlas_id,
                "capability": capability,
            }},
        )
        response.raise_for_status()
        return response.json()

    def execute_action(
        self,
        action_id: str,
        parameters: dict[str, Any],
    ) -> dict[str, Any]:
        """Execute a CRA-governed action."""
        if not self.session_id:
            self.create_session()

        # First resolve
        resolution = self.resolve(f"Execute {{action_id}}", action_id)
        resolution_id = resolution["resolution"]["resolution_id"]

        # Then execute
        response = self._client.post(
            f"{{self.runtime_url}}/v1/carp/execute",
            json={{
                "session_id": self.session_id,
                "resolution_id": resolution_id,
                "action_id": action_id,
                "parameters": parameters,
            }},
        )
        response.raise_for_status()
        return response.json()

    async def execute_action_async(
        self,
        action_id: str,
        parameters: dict[str, Any],
    ) -> dict[str, Any]:
        """Execute a CRA-governed action asynchronously."""
        if not self._async_client:
            self._async_client = httpx.AsyncClient(timeout=30.0)

        if not self.session_id:
            # Create session synchronously for simplicity
            self.create_session()

        # Resolve
        resolve_resp = await self._async_client.post(
            f"{{self.runtime_url}}/v1/carp/resolve",
            json={{
                "session_id": self.session_id,
                "goal": f"Execute {{action_id}}",
                "atlas_id": self.atlas_id,
                "capability": action_id,
            }},
        )
        resolve_resp.raise_for_status()
        resolution = resolve_resp.json()
        resolution_id = resolution["resolution"]["resolution_id"]

        # Execute
        exec_resp = await self._async_client.post(
            f"{{self.runtime_url}}/v1/carp/execute",
            json={{
                "session_id": self.session_id,
                "resolution_id": resolution_id,
                "action_id": action_id,
                "parameters": parameters,
            }},
        )
        exec_resp.raise_for_status()
        return exec_resp.json()

    def end_session(self) -> None:
        """End the current session."""
        if self.session_id:
            self._client.post(f"{{self.runtime_url}}/v1/sessions/{{self.session_id}}/end")
            self.session_id = None

    def __del__(self) -> None:
        """Cleanup on deletion."""
        self._client.close()


def create_cra_tools(
    runtime_url: str | None = None,
    principal_id: str = "langchain-agent",
) -> list[BaseTool]:
    """Create all CRA-backed tools.

    Args:
        runtime_url: CRA runtime URL
        principal_id: Agent principal ID

    Returns:
        List of LangChain tools
    """
    client = CRAClient(runtime_url)
    client.create_session(principal_id)

    tools: list[BaseTool] = []
{tool_definitions}
    return tools
'''

    def _generate_langgraph_agent(self, atlas: LoadedAtlas) -> str:
        """Generate LangGraph agent with CRA governance."""
        return f'''"""LangGraph agent with CRA governance for {atlas.manifest.name}.

This agent uses LangGraph for structured, stateful agent execution
while routing all tool calls through CRA.
"""

import os
from typing import Annotated, Any, Sequence, TypedDict

from langchain_core.messages import BaseMessage, HumanMessage, AIMessage
from langchain_openai import ChatOpenAI
from langgraph.graph import StateGraph, END
from langgraph.prebuilt import ToolNode

from cra_tools import create_cra_tools, CRAClient


class AgentState(TypedDict):
    """State for the CRA-governed agent."""
    messages: Annotated[Sequence[BaseMessage], "The messages in the conversation"]
    cra_session_id: str | None
    cra_trace_id: str | None


def create_cra_agent(
    model_name: str = "gpt-4",
    runtime_url: str | None = None,
):
    """Create a LangGraph agent with CRA governance.

    Args:
        model_name: OpenAI model to use
        runtime_url: CRA runtime URL

    Returns:
        Compiled LangGraph agent
    """
    # Initialize CRA client and tools
    cra_client = CRAClient(runtime_url, atlas_id="{atlas.manifest.id}")
    tools = create_cra_tools(runtime_url)

    # Create LLM with tools
    llm = ChatOpenAI(model=model_name)
    llm_with_tools = llm.bind_tools(tools)

    # Define the agent node
    def agent_node(state: AgentState) -> dict[str, Any]:
        """Process messages and decide on actions."""
        messages = state["messages"]

        # Ensure we have a CRA session
        if not state.get("cra_session_id"):
            session = cra_client.create_session()
            return {{
                "cra_session_id": session["session_id"],
                "cra_trace_id": session["trace_id"],
            }}

        # Get LLM response
        response = llm_with_tools.invoke(messages)
        return {{"messages": [response]}}

    # Define the tool node
    tool_node = ToolNode(tools)

    # Define routing logic
    def should_continue(state: AgentState) -> str:
        """Determine if we should continue to tools or end."""
        messages = state["messages"]
        last_message = messages[-1]

        if hasattr(last_message, "tool_calls") and last_message.tool_calls:
            return "tools"
        return END

    # Build the graph
    workflow = StateGraph(AgentState)

    # Add nodes
    workflow.add_node("agent", agent_node)
    workflow.add_node("tools", tool_node)

    # Set entry point
    workflow.set_entry_point("agent")

    # Add edges
    workflow.add_conditional_edges(
        "agent",
        should_continue,
        {{
            "tools": "tools",
            END: END,
        }},
    )
    workflow.add_edge("tools", "agent")

    # Compile
    return workflow.compile()


class CRAAgent:
    """High-level interface for the CRA-governed agent."""

    def __init__(
        self,
        model_name: str = "gpt-4",
        runtime_url: str | None = None,
    ):
        self.agent = create_cra_agent(model_name, runtime_url)
        self.cra_client = CRAClient(runtime_url, atlas_id="{atlas.manifest.id}")

    def run(self, message: str) -> str:
        """Run the agent with a user message.

        Args:
            message: User message

        Returns:
            Agent response
        """
        initial_state: AgentState = {{
            "messages": [HumanMessage(content=message)],
            "cra_session_id": None,
            "cra_trace_id": None,
        }}

        result = self.agent.invoke(initial_state)
        messages = result["messages"]

        # Get the last AI message
        for msg in reversed(messages):
            if isinstance(msg, AIMessage):
                return msg.content

        return "No response generated"

    def get_trace_id(self) -> str | None:
        """Get the current trace ID."""
        return self.cra_client.trace_id

    def end_session(self) -> None:
        """End the CRA session."""
        self.cra_client.end_session()


if __name__ == "__main__":
    agent = CRAAgent()
    response = agent.run("Hello, what can you help me with?")
    print(response)
    print(f"Trace ID: {{agent.get_trace_id()}}")
    agent.end_session()
'''

    def _generate_langchain_agent(self, atlas: LoadedAtlas) -> str:
        """Generate traditional LangChain agent."""
        return f'''"""LangChain agent with CRA governance for {atlas.manifest.name}.

Uses the classic LangChain agent architecture with CRA-backed tools.
"""

import os
from typing import Any

from langchain.agents import AgentExecutor, create_openai_functions_agent
from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder
from langchain_openai import ChatOpenAI

from cra_tools import create_cra_tools, CRAClient


def create_cra_agent(
    model_name: str = "gpt-4",
    runtime_url: str | None = None,
) -> AgentExecutor:
    """Create a LangChain agent with CRA governance.

    Args:
        model_name: OpenAI model to use
        runtime_url: CRA runtime URL

    Returns:
        AgentExecutor instance
    """
    # Create CRA-backed tools
    tools = create_cra_tools(runtime_url)

    # Create LLM
    llm = ChatOpenAI(model=model_name)

    # Create prompt
    prompt = ChatPromptTemplate.from_messages([
        ("system", """You are a helpful assistant powered by {atlas.manifest.name}.

Your actions are governed by CRA (Context Registry Agents). This means:
1. All actions are validated against policy before execution
2. Every action is logged to an immutable TRACE record
3. You must respect any denials or constraints

Available capabilities: {', '.join(atlas.manifest.capabilities)}

When you complete an action, mention the trace_id for reference."""),
        MessagesPlaceholder(variable_name="chat_history", optional=True),
        ("human", "{{input}}"),
        MessagesPlaceholder(variable_name="agent_scratchpad"),
    ])

    # Create agent
    agent = create_openai_functions_agent(llm, tools, prompt)

    # Create executor
    return AgentExecutor(
        agent=agent,
        tools=tools,
        verbose=True,
        handle_parsing_errors=True,
    )


class CRAAgent:
    """High-level interface for the CRA-governed agent."""

    def __init__(
        self,
        model_name: str = "gpt-4",
        runtime_url: str | None = None,
    ):
        self.executor = create_cra_agent(model_name, runtime_url)
        self.cra_client = CRAClient(runtime_url, atlas_id="{atlas.manifest.id}")
        self.chat_history: list = []

    def run(self, message: str) -> str:
        """Run the agent with a user message."""
        result = self.executor.invoke({{
            "input": message,
            "chat_history": self.chat_history,
        }})
        return result["output"]

    def get_trace_id(self) -> str | None:
        """Get the current trace ID."""
        return self.cra_client.trace_id

    def end_session(self) -> None:
        """End the CRA session."""
        self.cra_client.end_session()


if __name__ == "__main__":
    agent = CRAAgent()
    response = agent.run("Hello, what can you help me with?")
    print(response)
    print(f"Trace ID: {{agent.get_trace_id()}}")
    agent.end_session()
'''

    def _generate_main(self, atlas: LoadedAtlas) -> str:
        """Generate main example script."""
        return f'''#!/usr/bin/env python3
"""Example usage of the CRA-governed LangChain agent.

This demonstrates how to use the {atlas.manifest.name} Atlas
with LangChain/LangGraph and CRA governance.
"""

import os
import sys

from cra_agent import CRAAgent


def main():
    # Ensure CRA runtime is configured
    cra_url = os.getenv("CRA_RUNTIME_URL", "http://localhost:8420")
    print(f"Using CRA Runtime: {{cra_url}}")

    # Create agent
    agent = CRAAgent(
        model_name=os.getenv("OPENAI_MODEL", "gpt-4"),
        runtime_url=cra_url,
    )

    print(f"\\n{atlas.manifest.name} Agent")
    print("=" * 40)
    print("Type 'quit' to exit, 'trace' to see trace ID\\n")

    try:
        while True:
            user_input = input("You: ").strip()

            if not user_input:
                continue

            if user_input.lower() == "quit":
                break

            if user_input.lower() == "trace":
                print(f"Current Trace ID: {{agent.get_trace_id()}}")
                continue

            response = agent.run(user_input)
            print(f"\\nAssistant: {{response}}\\n")

    except KeyboardInterrupt:
        print("\\n")

    finally:
        print(f"Session Trace ID: {{agent.get_trace_id()}}")
        agent.end_session()
        print("Session ended.")


if __name__ == "__main__":
    main()
'''

    def _generate_requirements_content(self, use_langgraph: bool) -> str:
        """Generate requirements.txt content."""
        deps = [
            "cra>=0.1.0",
            "httpx>=0.25.0",
            "langchain>=0.1.0",
            "langchain-openai>=0.0.5",
        ]
        if use_langgraph:
            deps.append("langgraph>=0.0.20")
        return "\n".join(sorted(deps))

    def _generate_setup_instructions(self, atlas: LoadedAtlas, use_langgraph: bool) -> str:
        """Generate setup instructions."""
        framework = "LangGraph" if use_langgraph else "LangChain"
        return f"""# {atlas.manifest.name} {framework} Integration

## Prerequisites

1. Python 3.11+
2. OpenAI API key
3. CRA runtime running

## Setup

```bash
# Install dependencies
pip install -r requirements.txt

# Set environment variables
export CRA_RUNTIME_URL=http://localhost:8420
export OPENAI_API_KEY=your-key-here

# Run the agent
python main.py
```

## Files

- `cra_tools.py` - LangChain tools backed by CRA
- `cra_agent.py` - {framework} agent implementation
- `main.py` - Interactive example

## How It Works

1. Tools call CRA runtime for each action
2. CRA validates against Atlas policies
3. Actions are logged to TRACE
4. Results returned to the agent

## Customization

Edit `cra_agent.py` to:
- Modify the system prompt
- Add custom pre/post processing
- Integrate with other LangChain components
"""
