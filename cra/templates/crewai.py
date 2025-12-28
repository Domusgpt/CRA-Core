"""CrewAI template generator.

Generates CrewAI agents and crews with CRA governance.
"""

from typing import Any

from cra.core.atlas import LoadedAtlas
from cra.core.carp import Resolution
from cra.templates.base import GeneratedFile, GeneratedTemplate, TemplateGenerator


class CrewAIGenerator(TemplateGenerator):
    """Generator for CrewAI integration.

    Creates CrewAI agents and tools that are backed by CRA governance,
    ensuring all tool calls go through CARP resolution.
    """

    @property
    def framework_name(self) -> str:
        return "crewai"

    @property
    def framework_version(self) -> str:
        return "0.28.0"

    def generate(
        self,
        atlas: LoadedAtlas,
        resolution: Resolution | None = None,
        config: dict[str, Any] | None = None,
    ) -> GeneratedTemplate:
        """Generate CrewAI integration files.

        Args:
            atlas: The Atlas to generate from
            resolution: Optional resolution with allowed actions
            config: Optional config with:
                - crew_name: Name of the crew
                - model: Model to use (default: gpt-4)

        Returns:
            Generated CrewAI template
        """
        config = config or {}
        crew_name = config.get("crew_name", atlas.manifest.name.replace(" ", ""))

        files = []

        # Generate CRA tools for CrewAI
        tools_code = self._generate_tools_module(atlas)
        files.append(GeneratedFile(
            path="cra_tools.py",
            content=tools_code,
            description="CrewAI tools backed by CRA",
        ))

        # Generate agents module
        agents_code = self._generate_agents_module(atlas, crew_name)
        files.append(GeneratedFile(
            path="agents.py",
            content=agents_code,
            description="CrewAI agents with CRA governance",
        ))

        # Generate tasks module
        tasks_code = self._generate_tasks_module(atlas, crew_name)
        files.append(GeneratedFile(
            path="tasks.py",
            content=tasks_code,
            description="CrewAI tasks definition",
        ))

        # Generate crew module
        crew_code = self._generate_crew_module(atlas, crew_name)
        files.append(GeneratedFile(
            path="crew.py",
            content=crew_code,
            description="CrewAI crew definition",
        ))

        # Generate main example
        main_code = self._generate_main(atlas, crew_name)
        files.append(GeneratedFile(
            path="main.py",
            content=main_code,
            executable=True,
            description="Example usage script",
        ))

        # Generate requirements
        requirements = self._generate_requirements_content()
        files.append(GeneratedFile(
            path="requirements.txt",
            content=requirements,
            description="Python dependencies",
        ))

        return GeneratedTemplate(
            framework=self.framework_name,
            files=files,
            instructions=self._generate_setup_instructions(atlas, crew_name),
            dependencies=requirements.split("\n"),
        )

    def _generate_tools_module(self, atlas: LoadedAtlas) -> str:
        """Generate CrewAI tools backed by CRA."""
        capabilities = atlas.manifest.capabilities

        tool_classes = ""
        tool_instances = ""

        for cap in capabilities:
            class_name = "".join(word.title() for word in cap.split(".")) + "Tool"
            tool_name = cap.replace(".", "_")

            tool_classes += f'''
class {class_name}(BaseTool):
    """Tool for {cap} capability."""

    name: str = "{tool_name}"
    description: str = "Execute the {cap} action through CRA governance"

    def _run(self, **kwargs: Any) -> str:
        """Execute the tool."""
        client = get_cra_client()
        result = client.execute_action("{cap}", kwargs)
        return json.dumps(result, indent=2)

'''
            tool_instances += f"    {class_name}(),\n"

        return f'''"""CRA-backed CrewAI tools for {atlas.manifest.name}.

These tools integrate with CRA governance to ensure all actions
are validated, logged, and auditable.
"""

import json
import os
from typing import Any

import httpx
from crewai_tools import BaseTool


class CRAClient:
    """Client for CRA runtime communication."""

    _instance: "CRAClient | None" = None

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

    @classmethod
    def get_instance(cls) -> "CRAClient":
        """Get singleton instance."""
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def create_session(self, principal_id: str = "crewai-agent") -> dict[str, Any]:
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

    def ensure_session(self) -> None:
        """Ensure we have an active session."""
        if not self.session_id:
            self.create_session()

    def resolve(self, goal: str, capability: str) -> dict[str, Any]:
        """Resolve context and permissions for a goal."""
        self.ensure_session()

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
        self.ensure_session()

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

    def end_session(self) -> None:
        """End the current session."""
        if self.session_id:
            self._client.post(f"{{self.runtime_url}}/v1/sessions/{{self.session_id}}/end")
            self.session_id = None


def get_cra_client() -> CRAClient:
    """Get the CRA client singleton."""
    return CRAClient.get_instance()


# Tool definitions
{tool_classes}

def get_all_tools() -> list[BaseTool]:
    """Get all CRA-backed tools."""
    # Ensure session is created
    get_cra_client().ensure_session()

    return [
{tool_instances}    ]
'''

    def _generate_agents_module(self, atlas: LoadedAtlas, crew_name: str) -> str:
        """Generate CrewAI agents module."""
        return f'''"""CrewAI agents for {atlas.manifest.name}.

These agents are governed by CRA and use CRA-backed tools.
"""

import os

from crewai import Agent

from cra_tools import get_all_tools


def create_researcher_agent() -> Agent:
    """Create a researcher agent.

    This agent analyzes requirements and determines
    what actions are needed.
    """
    return Agent(
        role="Research Analyst",
        goal="Analyze requirements and determine the best course of action",
        backstory="""You are an expert analyst working with the {atlas.manifest.name} system.
You carefully analyze requirements and plan actions while respecting
the governance constraints of CRA. You always verify what actions
are permitted before recommending them.""",
        verbose=True,
        allow_delegation=False,
        tools=get_all_tools(),
    )


def create_executor_agent() -> Agent:
    """Create an executor agent.

    This agent executes actions through CRA governance.
    """
    return Agent(
        role="Action Executor",
        goal="Execute approved actions accurately and report results",
        backstory="""You are a skilled executor working with the {atlas.manifest.name} system.
You execute actions that have been approved through CRA governance,
ensuring each action is properly logged and traceable. You report
outcomes clearly, including any trace IDs for verification.""",
        verbose=True,
        allow_delegation=False,
        tools=get_all_tools(),
    )


def create_reviewer_agent() -> Agent:
    """Create a reviewer agent.

    This agent reviews outcomes and ensures compliance.
    """
    return Agent(
        role="Compliance Reviewer",
        goal="Review action outcomes and ensure governance compliance",
        backstory="""You are a compliance officer for the {atlas.manifest.name} system.
You review all actions taken, verify they were properly authorized,
and ensure the outcomes meet the expected governance standards.
You flag any concerns and provide clear audit summaries.""",
        verbose=True,
        allow_delegation=False,
        tools=[],  # Reviewer doesn't execute actions directly
    )
'''

    def _generate_tasks_module(self, atlas: LoadedAtlas, crew_name: str) -> str:
        """Generate CrewAI tasks module."""
        return f'''"""CrewAI tasks for {atlas.manifest.name}.

These tasks define the workflow for CRA-governed operations.
"""

from crewai import Task, Agent


def create_analysis_task(
    agent: Agent,
    objective: str,
) -> Task:
    """Create an analysis task.

    Args:
        agent: The researcher agent
        objective: The objective to analyze

    Returns:
        Analysis task
    """
    return Task(
        description=f"""Analyze the following objective and determine what actions are needed:

Objective: {{objective}}

Steps:
1. Break down the objective into specific actions
2. Identify which CRA capabilities are required
3. Check if any actions might be denied by policy
4. Create a clear action plan

Available capabilities: {', '.join(atlas.manifest.capabilities)}

Output a structured plan with:
- Required actions
- Expected parameters
- Potential risks or constraints""",
        expected_output="A structured action plan with required capabilities and parameters",
        agent=agent,
    )


def create_execution_task(
    agent: Agent,
    action_plan: str,
) -> Task:
    """Create an execution task.

    Args:
        agent: The executor agent
        action_plan: The plan to execute

    Returns:
        Execution task
    """
    return Task(
        description=f"""Execute the following action plan through CRA governance:

{{action_plan}}

For each action:
1. Call the appropriate CRA tool
2. Capture the result and trace_id
3. Handle any errors gracefully
4. Report the outcome

Remember: All actions are logged to TRACE and must be authorized.""",
        expected_output="Execution report with results and trace IDs for each action",
        agent=agent,
    )


def create_review_task(
    agent: Agent,
    execution_report: str,
) -> Task:
    """Create a review task.

    Args:
        agent: The reviewer agent
        execution_report: The execution report to review

    Returns:
        Review task
    """
    return Task(
        description=f"""Review the following execution report for compliance:

{{execution_report}}

Verify:
1. All actions were properly authorized
2. No policy violations occurred
3. Results match expected outcomes
4. Trace IDs are present for all actions

Provide:
- Compliance status (pass/fail)
- Summary of actions taken
- Any concerns or recommendations""",
        expected_output="Compliance review with status, summary, and recommendations",
        agent=agent,
    )
'''

    def _generate_crew_module(self, atlas: LoadedAtlas, crew_name: str) -> str:
        """Generate CrewAI crew module."""
        return f'''"""CrewAI crew for {atlas.manifest.name}.

This crew orchestrates CRA-governed operations with
multiple specialized agents.
"""

from crewai import Crew, Process

from agents import (
    create_researcher_agent,
    create_executor_agent,
    create_reviewer_agent,
)
from tasks import (
    create_analysis_task,
    create_execution_task,
    create_review_task,
)
from cra_tools import get_cra_client


class {crew_name}Crew:
    """Crew for {atlas.manifest.name} operations.

    This crew uses CRA governance for all actions.
    """

    def __init__(self):
        self.researcher = create_researcher_agent()
        self.executor = create_executor_agent()
        self.reviewer = create_reviewer_agent()

    def run(self, objective: str) -> str:
        """Run the crew with an objective.

        Args:
            objective: The objective to accomplish

        Returns:
            Final output from the crew
        """
        # Create tasks
        analysis_task = create_analysis_task(self.researcher, objective)
        execution_task = create_execution_task(self.executor, "{{analysis_task.output}}")
        review_task = create_review_task(self.reviewer, "{{execution_task.output}}")

        # Create crew
        crew = Crew(
            agents=[self.researcher, self.executor, self.reviewer],
            tasks=[analysis_task, execution_task, review_task],
            process=Process.sequential,
            verbose=True,
        )

        # Run crew
        result = crew.kickoff()

        return str(result)

    def get_trace_id(self) -> str | None:
        """Get the current CRA trace ID."""
        return get_cra_client().trace_id

    def end_session(self) -> None:
        """End the CRA session."""
        get_cra_client().end_session()


def create_simple_crew() -> Crew:
    """Create a simple single-agent crew.

    For simpler use cases that don't need the full
    researcher-executor-reviewer workflow.
    """
    executor = create_executor_agent()

    return Crew(
        agents=[executor],
        tasks=[],  # Tasks added dynamically
        process=Process.sequential,
        verbose=True,
    )
'''

    def _generate_main(self, atlas: LoadedAtlas, crew_name: str) -> str:
        """Generate main example script."""
        return f'''#!/usr/bin/env python3
"""Example usage of the CRA-governed CrewAI crew.

This demonstrates how to use the {atlas.manifest.name} Atlas
with CrewAI and CRA governance.
"""

import os
import sys

from crew import {crew_name}Crew


def main():
    # Ensure CRA runtime is configured
    cra_url = os.getenv("CRA_RUNTIME_URL", "http://localhost:8420")
    print(f"Using CRA Runtime: {{cra_url}}")

    # Create crew
    crew = {crew_name}Crew()

    print(f"\\n{atlas.manifest.name} CrewAI Crew")
    print("=" * 40)

    # Example objective
    if len(sys.argv) > 1:
        objective = " ".join(sys.argv[1:])
    else:
        objective = input("Enter objective: ").strip()

    if not objective:
        print("No objective provided")
        return

    print(f"\\nObjective: {{objective}}")
    print("-" * 40)

    try:
        result = crew.run(objective)
        print("\\n" + "=" * 40)
        print("RESULT:")
        print("=" * 40)
        print(result)

    except Exception as e:
        print(f"\\nError: {{e}}")

    finally:
        print(f"\\nSession Trace ID: {{crew.get_trace_id()}}")
        crew.end_session()
        print("Session ended.")


if __name__ == "__main__":
    main()
'''

    def _generate_requirements_content(self) -> str:
        """Generate requirements.txt content."""
        deps = [
            "cra>=0.1.0",
            "crewai>=0.28.0",
            "crewai-tools>=0.1.0",
            "httpx>=0.25.0",
        ]
        return "\n".join(sorted(deps))

    def _generate_setup_instructions(self, atlas: LoadedAtlas, crew_name: str) -> str:
        """Generate setup instructions."""
        return f"""# {atlas.manifest.name} CrewAI Integration

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

# Run the crew
python main.py "Your objective here"
```

## Files

- `cra_tools.py` - CrewAI tools backed by CRA
- `agents.py` - Agent definitions
- `tasks.py` - Task templates
- `crew.py` - {crew_name}Crew definition
- `main.py` - Example usage

## Architecture

The crew consists of three agents:

1. **Researcher** - Analyzes objectives and plans actions
2. **Executor** - Executes CRA-governed actions
3. **Reviewer** - Ensures compliance and audits results

## Customization

- Edit `agents.py` to modify agent roles and backstories
- Edit `tasks.py` to customize task workflows
- Edit `crew.py` to change the process flow

## Governance

All tool calls go through CRA:
1. Actions are resolved against Atlas policies
2. Execution is logged to TRACE
3. Results include trace_id for auditing
"""
