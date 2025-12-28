# Integration Guide

Complete guide to integrating CRA with AI agent frameworks and platforms.

---

## Table of Contents

- [Overview](#overview)
- [OpenAI Integration](#openai-integration)
- [LangChain Integration](#langchain-integration)
- [LangGraph Integration](#langgraph-integration)
- [CrewAI Integration](#crewai-integration)
- [Anthropic Integration](#anthropic-integration)
- [Google ADK Integration](#google-adk-integration)
- [MCP Integration](#mcp-integration)
- [Custom Integrations](#custom-integrations)
- [Best Practices](#best-practices)

---

## Overview

CRA provides multiple integration patterns:

1. **Middleware** — Drop-in governance layer for existing code
2. **Adapters** — Convert Atlas capabilities to platform-native formats
3. **Templates** — Generate ready-to-use integration code

### Integration Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Your Agent Code                          │
├──────────────────────────────────────────────────────────────┤
│                    CRA Middleware                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │   OpenAI    │  │  LangChain  │  │      CrewAI         │   │
│  │  Middleware │  │  Middleware │  │     Middleware      │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│                    CRA Core                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────────────┐   │
│  │  CARP   │  │  TRACE  │  │  Atlas  │  │   Policies     │   │
│  └─────────┘  └─────────┘  └─────────┘  └────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

---

## OpenAI Integration

### Using OpenAI Middleware

The OpenAI middleware provides CRA governance for OpenAI function calling.

```python
from openai import OpenAI
from cra.middleware import OpenAIMiddleware

# Initialize clients
client = OpenAI()
middleware = OpenAIMiddleware()

# Get CRA-governed tools
tools = middleware.get_tools(
    goal="Help user analyze sales data",
    atlas_id="com.example.data-analytics",
)

# Create completion with tools
response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "system", "content": middleware.get_system_message()},
        {"role": "user", "content": "Show me last month's revenue"},
    ],
    tools=tools,
)

# Handle tool calls through CRA
if response.choices[0].message.tool_calls:
    messages = [
        {"role": "system", "content": middleware.get_system_message()},
        {"role": "user", "content": "Show me last month's revenue"},
        response.choices[0].message,
    ]

    for tool_call in response.choices[0].message.tool_calls:
        # Execute through CRA (policy-checked, traced)
        result = middleware.handle_tool_call(tool_call)

        # Add tool response
        messages.append(
            middleware.create_tool_message(tool_call.id, result)
        )

    # Continue conversation
    response = client.chat.completions.create(
        model="gpt-4",
        messages=messages,
    )

print(response.choices[0].message.content)
print(f"Trace ID: {middleware.get_trace_id()}")
```

### Using OpenAI Adapter

Generate OpenAI-compatible tools from an Atlas:

```bash
cra atlas emit com.example.data-analytics -p openai -o tools.json
```

```python
import json
from openai import OpenAI

# Load generated tools
with open("tools.json") as f:
    tools = json.load(f)["tools"]

client = OpenAI()
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Run a query"}],
    tools=tools,
)
```

### OpenAI GPT Actions Template

Generate a complete GPT Actions configuration:

```bash
cra template generate openai-gpt \
    --atlas com.example.data-analytics \
    --base-url https://api.example.com \
    --output gpt-action.json
```

This generates:
- OpenAPI specification
- Authentication configuration
- Privacy policy template
- Instructions for GPT Builder

---

## LangChain Integration

### Using LangChain Middleware

```python
from langchain_openai import ChatOpenAI
from langchain.agents import create_openai_functions_agent, AgentExecutor
from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder
from cra.middleware import LangChainMiddleware

# Initialize middleware
middleware = LangChainMiddleware()

# Get CRA-governed tools as LangChain Tool objects
tools = middleware.get_tools(
    goal="Help with customer support",
    atlas_id="com.example.customer-support",
)

# Create LangChain agent
llm = ChatOpenAI(model="gpt-4")

prompt = ChatPromptTemplate.from_messages([
    ("system", middleware._get_system_prompt()),
    MessagesPlaceholder(variable_name="chat_history", optional=True),
    ("human", "{input}"),
    MessagesPlaceholder(variable_name="agent_scratchpad"),
])

agent = create_openai_functions_agent(llm, tools, prompt)
executor = AgentExecutor(agent=agent, tools=tools, verbose=True)

# Run agent
result = executor.invoke({
    "input": "Look up ticket #12345",
    "chat_history": [],
})

print(result["output"])
print(f"Trace ID: {middleware.get_trace_id()}")
```

### Quick Start with get_runnable

```python
from cra.middleware import LangChainMiddleware

middleware = LangChainMiddleware()

# Get a ready-to-use runnable
runnable = middleware.get_runnable(
    goal="Customer support agent",
    atlas_id="com.example.customer-support",
    model="gpt-4",
)

# Use it
result = runnable.invoke({"input": "Check order status for customer@example.com"})
print(result["output"])
```

### LangChain Template

Generate a complete LangChain integration:

```bash
cra template generate langchain \
    --atlas com.example.customer-support \
    --output langchain_agent.py
```

Generated code includes:
- Tool definitions with proper schemas
- Agent setup with CRA governance
- Error handling and tracing

---

## LangGraph Integration

### State-Based Agent with CRA

```python
from typing import TypedDict, Annotated
from langgraph.graph import StateGraph, END
from langchain_openai import ChatOpenAI
from cra.middleware import LangChainMiddleware

# Define state
class AgentState(TypedDict):
    messages: list
    trace_id: str | None

# Initialize
middleware = LangChainMiddleware()
tools = middleware.get_tools(
    goal="Data analysis",
    atlas_id="com.example.data-analytics",
)
llm = ChatOpenAI(model="gpt-4").bind_tools(tools)

# Define nodes
def agent_node(state: AgentState):
    response = llm.invoke(state["messages"])
    return {
        "messages": state["messages"] + [response],
        "trace_id": str(middleware.get_trace_id()),
    }

def tool_node(state: AgentState):
    last_message = state["messages"][-1]
    results = []

    for tool_call in last_message.tool_calls:
        # Execute through CRA middleware
        action_id = tool_call["name"].replace("_", ".")
        result = middleware.execute(action_id, tool_call["args"])

        results.append({
            "role": "tool",
            "content": str(result),
            "tool_call_id": tool_call["id"],
        })

    return {"messages": state["messages"] + results}

def should_continue(state: AgentState):
    last_message = state["messages"][-1]
    if hasattr(last_message, "tool_calls") and last_message.tool_calls:
        return "tools"
    return END

# Build graph
workflow = StateGraph(AgentState)
workflow.add_node("agent", agent_node)
workflow.add_node("tools", tool_node)
workflow.set_entry_point("agent")
workflow.add_conditional_edges("agent", should_continue, {"tools": "tools", END: END})
workflow.add_edge("tools", "agent")

app = workflow.compile()

# Run
result = app.invoke({
    "messages": [{"role": "user", "content": "Show me top customers"}],
    "trace_id": None,
})
```

### LangGraph Template

```bash
cra template generate langgraph \
    --atlas com.example.data-analytics \
    --output langgraph_agent.py
```

---

## CrewAI Integration

### Creating CRA-Governed Crews

```python
from crewai import Agent, Task, Crew
from cra.middleware import CRAMiddleware

# Initialize CRA
middleware = CRAMiddleware()
middleware.resolve(
    goal="Customer support operations",
    atlas_id="com.example.customer-support",
)

# Define CRA-wrapped tools
def lookup_ticket(ticket_id: str) -> str:
    """Look up a support ticket."""
    result = middleware.execute("ticket.lookup", {"ticket_id": ticket_id})
    return str(result.get("result", result.get("error")))

def update_ticket(ticket_id: str, status: str, notes: str = "") -> str:
    """Update a support ticket."""
    result = middleware.execute("ticket.update", {
        "ticket_id": ticket_id,
        "status": status,
        "notes": notes,
    })
    return str(result.get("result", result.get("error")))

def search_knowledge_base(query: str) -> str:
    """Search the knowledge base."""
    result = middleware.execute("kb.search", {"query": query})
    return str(result.get("result", result.get("error")))

# Create agents
support_agent = Agent(
    role="Customer Support Specialist",
    goal="Resolve customer issues efficiently",
    backstory="Expert support agent with deep product knowledge",
    tools=[lookup_ticket, update_ticket, search_knowledge_base],
    verbose=True,
)

# Create tasks
resolve_task = Task(
    description="Look up ticket #12345 and resolve the customer's issue",
    agent=support_agent,
    expected_output="Resolution summary with actions taken",
)

# Create and run crew
crew = Crew(
    agents=[support_agent],
    tasks=[resolve_task],
    verbose=True,
)

result = crew.kickoff()
print(result)
print(f"Trace ID: {middleware.get_trace_id()}")
```

### CrewAI Template

```bash
cra template generate crewai \
    --atlas com.example.customer-support \
    --output crew_agent.py
```

Generated code includes:
- Tool functions with CRA execution
- Agent definitions with proper roles
- Crew setup with governance

---

## Anthropic Integration

### Using Anthropic Tool Use

```python
import anthropic
import json

# Load Atlas adapter
with open("adapters/anthropic.skill.md") as f:
    skill_content = f.read()

# Or generate it
# cra atlas emit com.example.atlas -p anthropic -o skill.md

client = anthropic.Anthropic()

response = client.messages.create(
    model="claude-3-opus-20240229",
    max_tokens=1024,
    system=skill_content,  # Atlas context as system prompt
    messages=[
        {"role": "user", "content": "Help me with..."}
    ],
    tools=[
        {
            "name": "resource_create",
            "description": "Create a new resource",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "type": {"type": "string", "enum": ["typeA", "typeB"]}
                },
                "required": ["name", "type"]
            }
        }
    ]
)
```

### With CRA Middleware

```python
from anthropic import Anthropic
from cra.middleware import CRAMiddleware
import json

client = Anthropic()
middleware = CRAMiddleware()

# Resolve capabilities
resolution = middleware.resolve(
    goal="Manage resources",
    atlas_id="com.example.resources",
)

# Convert to Anthropic tools
tools = []
for action in resolution.allowed_actions:
    tools.append({
        "name": action["action_id"].replace(".", "_"),
        "description": action.get("description", ""),
        "input_schema": action.get("schema", {"type": "object", "properties": {}}),
    })

# Make request
response = client.messages.create(
    model="claude-3-opus-20240229",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Create a resource"}],
    tools=tools,
)

# Handle tool use
for block in response.content:
    if block.type == "tool_use":
        action_id = block.name.replace("_", ".")
        result = middleware.execute(action_id, block.input)
        print(f"Executed {action_id}: {result}")
```

---

## Google ADK Integration

### Generate ADK Configuration

```bash
cra atlas emit com.example.atlas -p google_adk -o adk_config.json
```

### Use with Google ADK

```python
import json

# Load ADK configuration
with open("adk_config.json") as f:
    adk_config = json.load(f)

# Use with Google's Agent Development Kit
# (Follow Google ADK documentation for specific usage)
```

---

## MCP Integration

### Generate MCP Server Configuration

```bash
cra atlas emit com.example.atlas -p mcp -o mcp_server.json
```

### MCP Server Setup

The generated configuration includes:
- Server metadata
- Tool definitions
- Resource definitions
- Prompt templates

```json
{
  "name": "com.example.atlas",
  "version": "1.0.0",
  "description": "Atlas MCP Server",
  "tools": [
    {
      "name": "resource_create",
      "description": "Create a new resource",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": {"type": "string"}
        },
        "required": ["name"]
      }
    }
  ],
  "resources": [],
  "prompts": []
}
```

---

## Custom Integrations

### Direct API Integration

For frameworks not covered by built-in middleware:

```python
import httpx
from uuid import uuid4

class CRAClient:
    def __init__(self, base_url: str = "http://localhost:8420"):
        self.base_url = base_url
        self.client = httpx.Client()
        self.session_id = None

    def start_session(self, agent_id: str, goal: str, atlas_id: str = None):
        """Start a CRA session."""
        response = self.client.post(
            f"{self.base_url}/v1/sessions",
            json={
                "agent_id": agent_id,
                "goal": goal,
                "atlas_id": atlas_id,
            }
        )
        data = response.json()
        self.session_id = data["session_id"]
        return data

    def resolve(self, goal: str, atlas_id: str = None, capability: str = None):
        """Resolve available actions via CARP."""
        response = self.client.post(
            f"{self.base_url}/v1/resolve",
            json={
                "session_id": self.session_id,
                "goal": goal,
                "atlas_id": atlas_id,
                "capability": capability,
            }
        )
        return response.json()

    def execute(self, action_id: str, parameters: dict):
        """Execute an action."""
        response = self.client.post(
            f"{self.base_url}/v1/execute",
            json={
                "session_id": self.session_id,
                "action_id": action_id,
                "parameters": parameters,
            }
        )
        return response.json()

    def get_trace(self):
        """Get the trace for this session."""
        response = self.client.get(
            f"{self.base_url}/v1/traces/{self.session_id}"
        )
        return response.json()


# Usage
cra = CRAClient()
cra.start_session("my-agent", "Help user", "com.example.atlas")
resolution = cra.resolve("Perform action")

for action in resolution["allowed_actions"]:
    print(f"Available: {action['action_id']}")

result = cra.execute("resource.create", {"name": "test"})
print(result)
```

### Building a Custom Middleware

```python
from cra.middleware.base import CRAMiddleware

class MyFrameworkMiddleware(CRAMiddleware):
    """Custom middleware for MyFramework."""

    def get_tools(self, goal: str, atlas_id: str = None):
        """Convert CRA actions to MyFramework tools."""
        self.resolve(goal, atlas_id)

        tools = []
        for action in self._resolution.allowed_actions:
            # Convert to your framework's format
            tool = self._convert_to_my_format(action)
            tools.append(tool)

        return tools

    def _convert_to_my_format(self, action: dict):
        """Convert CRA action to MyFramework tool format."""
        return {
            "name": action["action_id"],
            "handler": lambda params: self.execute(action["action_id"], params),
            # Add framework-specific fields
        }

    def wrap_tool_call(self, tool_name: str, parameters: dict):
        """Wrap a tool call with CRA governance."""
        action_id = tool_name  # Adjust mapping as needed
        return self.execute(action_id, parameters)
```

---

## Best Practices

### 1. Always Use Middleware for Production

Middleware provides:
- Automatic policy enforcement
- Complete tracing
- Error handling
- Denial list enforcement

```python
# Good: Use middleware
middleware = OpenAIMiddleware()
tools = middleware.get_tools(goal, atlas_id)

# Avoid: Direct tool definitions without CRA
tools = [{"type": "function", ...}]  # No governance
```

### 2. Include Trace IDs in Responses

Always surface trace IDs for auditability:

```python
result = middleware.execute(action_id, params)
response = {
    "result": result,
    "trace_id": str(middleware.get_trace_id()),
}
```

### 3. Handle Denials Gracefully

Check for denied actions and inform users:

```python
resolution = middleware.resolve(goal, atlas_id)

if resolution.denylist:
    for rule in resolution.denylist:
        print(f"Denied: {rule['pattern']} - {rule['reason']}")
```

### 4. Use Appropriate Atlases

Load the right Atlas for the task:

```python
# Specific Atlas for specific domain
tools = middleware.get_tools(
    goal="Customer support",
    atlas_id="com.example.customer-support",  # Specific
)

# Capability filter for focused tools
tools = middleware.get_tools(
    goal="Ticket lookup only",
    atlas_id="com.example.customer-support",
    capability="ticket.read",  # Only read operations
)
```

### 5. Test Integrations Thoroughly

Use golden traces to verify behavior:

```bash
# Record expected behavior
cra replay --record --output expected_trace.json

# Verify integration produces same trace
cra replay --manifest expected_trace.json --compare
```

### 6. Monitor in Production

Enable observability:

```python
# In production configuration
CRA_OBSERVABILITY__OTEL_ENABLED=true
CRA_OBSERVABILITY__SIEM_ENABLED=true
CRA_OBSERVABILITY__METRICS_ENABLED=true
```

### 7. Version Your Atlases

Pin Atlas versions in production:

```python
# Specify version
tools = middleware.get_tools(
    goal="Customer support",
    atlas_id="com.example.customer-support@1.2.0",  # Pinned version
)
```

---

## Troubleshooting Integrations

### Tool Not Found

```python
# Check available tools
resolution = middleware.resolve(goal, atlas_id)
print("Available actions:", [a["action_id"] for a in resolution.allowed_actions])
```

### Policy Violation

```python
result = middleware.execute(action_id, params)
if result.get("status") == "denied":
    print(f"Policy violation: {result.get('error')}")
```

### Schema Mismatch

```python
# Validate parameters match schema
action = next(a for a in resolution.allowed_actions if a["action_id"] == action_id)
schema = action.get("schema", {})
print(f"Required fields: {schema.get('required', [])}")
```

---

*For more information, see the [API Reference](API.md) and [Atlas Development Guide](ATLASES.md).*
