"""OpenAI GPT Action template generator.

Generates GPT Action configuration for OpenAI's custom GPTs.
"""

import json
from typing import Any

from cra.core.atlas import LoadedAtlas
from cra.core.carp import Resolution
from cra.templates.base import GeneratedFile, GeneratedTemplate, TemplateGenerator


class OpenAIGPTActionGenerator(TemplateGenerator):
    """Generator for OpenAI GPT Actions.

    Creates the configuration files needed to set up
    a custom GPT with CRA-governed actions.
    """

    @property
    def framework_name(self) -> str:
        return "openai_gpt"

    @property
    def framework_version(self) -> str:
        return "2024-01"

    def generate(
        self,
        atlas: LoadedAtlas,
        resolution: Resolution | None = None,
        config: dict[str, Any] | None = None,
    ) -> GeneratedTemplate:
        """Generate GPT Action configuration.

        Args:
            atlas: The Atlas to generate from
            resolution: Optional resolution with allowed actions
            config: Optional config with:
                - api_base_url: Base URL for the API
                - auth_type: Authentication type (none, api_key, oauth)

        Returns:
            Generated GPT Action configuration
        """
        config = config or {}
        api_base_url = config.get("api_base_url", "https://api.example.com")
        auth_type = config.get("auth_type", "api_key")

        files = []

        # Generate OpenAPI spec for GPT Actions
        openapi_spec = self._generate_openapi_spec(atlas, api_base_url, auth_type)
        files.append(GeneratedFile(
            path="openapi.json",
            content=json.dumps(openapi_spec, indent=2),
            description="OpenAPI specification for GPT Actions",
        ))

        # Generate GPT instructions
        instructions = self._generate_gpt_instructions(atlas)
        files.append(GeneratedFile(
            path="instructions.md",
            content=instructions,
            description="GPT system instructions",
        ))

        # Generate privacy policy template
        privacy_policy = self._generate_privacy_policy(atlas)
        files.append(GeneratedFile(
            path="privacy-policy.md",
            content=privacy_policy,
            description="Privacy policy template for GPT",
        ))

        # Generate CRA integration server
        server_code = self._generate_server_code(atlas, api_base_url)
        files.append(GeneratedFile(
            path="server.py",
            content=server_code,
            description="FastAPI server bridging GPT Actions to CRA",
        ))

        return GeneratedTemplate(
            framework=self.framework_name,
            files=files,
            instructions=self._generate_setup_instructions(atlas),
            dependencies=[
                "fastapi>=0.109.0",
                "uvicorn>=0.27.0",
                "httpx>=0.25.0",
                "cra>=0.1.0",
            ],
        )

    def _generate_openapi_spec(
        self,
        atlas: LoadedAtlas,
        api_base_url: str,
        auth_type: str,
    ) -> dict[str, Any]:
        """Generate OpenAPI specification for GPT Actions."""
        spec: dict[str, Any] = {
            "openapi": "3.1.0",
            "info": {
                "title": f"{atlas.manifest.name} API",
                "description": atlas.manifest.description or f"CRA-governed API for {atlas.manifest.name}",
                "version": atlas.manifest.version,
            },
            "servers": [
                {"url": api_base_url}
            ],
            "paths": {},
            "components": {
                "schemas": {},
            },
        }

        # Add security scheme based on auth type
        if auth_type == "api_key":
            spec["components"]["securitySchemes"] = {
                "ApiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key",
                }
            }
            spec["security"] = [{"ApiKeyAuth": []}]
        elif auth_type == "oauth":
            spec["components"]["securitySchemes"] = {
                "OAuth2": {
                    "type": "oauth2",
                    "flows": {
                        "authorizationCode": {
                            "authorizationUrl": f"{api_base_url}/oauth/authorize",
                            "tokenUrl": f"{api_base_url}/oauth/token",
                            "scopes": {
                                cap: f"Access to {cap}"
                                for cap in atlas.manifest.capabilities
                            }
                        }
                    }
                }
            }
            spec["security"] = [{"OAuth2": atlas.manifest.capabilities}]

        # Generate paths for each capability
        for capability in atlas.manifest.capabilities:
            path = f"/actions/{capability.replace('.', '/')}"
            spec["paths"][path] = {
                "post": {
                    "operationId": capability.replace(".", "_"),
                    "summary": f"Execute {capability}",
                    "description": f"Execute the {capability} action through CRA governance",
                    "requestBody": {
                        "required": True,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "parameters": {
                                            "type": "object",
                                            "description": "Action parameters",
                                        },
                                    },
                                },
                            },
                        },
                    },
                    "responses": {
                        "200": {
                            "description": "Action executed successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "success": {"type": "boolean"},
                                            "result": {"type": "object"},
                                            "trace_id": {"type": "string"},
                                        },
                                    },
                                },
                            },
                        },
                        "403": {
                            "description": "Action not permitted by CRA policy",
                        },
                    },
                },
            }

        return spec

    def _generate_gpt_instructions(self, atlas: LoadedAtlas) -> str:
        """Generate GPT system instructions."""
        capabilities_list = "\n".join(f"- {cap}" for cap in atlas.manifest.capabilities)

        return f"""# {atlas.manifest.name}

{atlas.manifest.description or "A CRA-governed assistant."}

## Governance

This GPT operates under CRA (Context Registry Agents) governance. All actions are:
1. Validated against policy before execution
2. Logged to an immutable TRACE record
3. Subject to the constraints defined in the Atlas

## Available Capabilities

{capabilities_list}

## Important Rules

1. **Always use the provided actions** - Do not attempt to simulate or bypass actions
2. **Respect denials** - If an action is denied, explain why and suggest alternatives
3. **Honor approvals** - Some actions require explicit user approval before execution
4. **Cite TRACE** - When confirming action completion, reference the trace_id

## Error Handling

If an action fails:
1. Report the error clearly to the user
2. Provide the trace_id for debugging
3. Suggest alternative approaches if available
"""

    def _generate_privacy_policy(self, atlas: LoadedAtlas) -> str:
        """Generate privacy policy template."""
        return f"""# Privacy Policy for {atlas.manifest.name} GPT

Last updated: [DATE]

## Overview

This GPT ({atlas.manifest.name}) uses CRA (Context Registry Agents) governance
to manage actions and data access.

## Data Collection

- **Action Requests**: All action requests are logged to an immutable TRACE record
- **Parameters**: Action parameters may be logged for audit purposes
- **User Identifiers**: Session identifiers are maintained for request correlation

## Data Usage

Data collected is used for:
- Action execution and response generation
- Audit and compliance requirements
- Service improvement and debugging

## Data Retention

TRACE records are retained according to your organization's retention policy.
Default retention is 30 days.

## Your Rights

You may request:
- Access to your TRACE records
- Deletion of your session data (subject to compliance requirements)

## Contact

[Your contact information]
"""

    def _generate_server_code(self, atlas: LoadedAtlas, api_base_url: str) -> str:
        """Generate FastAPI server code."""
        return f'''"""CRA Bridge Server for {atlas.manifest.name} GPT Actions.

This server bridges OpenAI GPT Actions to CRA governance.
"""

import os
from typing import Any
from uuid import uuid4

import httpx
from fastapi import FastAPI, HTTPException, Header, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel

app = FastAPI(
    title="{atlas.manifest.name} CRA Bridge",
    version="{atlas.manifest.version}",
)

CRA_RUNTIME_URL = os.getenv("CRA_RUNTIME_URL", "http://localhost:8420")
API_KEY = os.getenv("API_KEY", "")


class ActionRequest(BaseModel):
    """Request to execute an action."""
    parameters: dict[str, Any] = {{}}


class ActionResponse(BaseModel):
    """Response from action execution."""
    success: bool
    result: dict[str, Any] | None = None
    error: str | None = None
    trace_id: str


async def verify_api_key(x_api_key: str = Header(None)) -> bool:
    """Verify API key if configured."""
    if API_KEY and x_api_key != API_KEY:
        raise HTTPException(status_code=401, detail="Invalid API key")
    return True


@app.post("/actions/{{capability:path}}")
async def execute_action(
    capability: str,
    request: ActionRequest,
    x_api_key: str = Header(None),
) -> ActionResponse:
    """Execute a CRA-governed action.

    1. Creates a session with CRA runtime
    2. Resolves context and permissions
    3. Executes the action if permitted
    4. Returns result with trace_id
    """
    await verify_api_key(x_api_key)

    async with httpx.AsyncClient() as client:
        # Create session
        session_resp = await client.post(
            f"{{CRA_RUNTIME_URL}}/v1/sessions",
            json={{
                "principal": {{"type": "agent", "id": "gpt-action"}},
                "scopes": [capability],
            }},
        )
        if session_resp.status_code != 200:
            return ActionResponse(
                success=False,
                error="Failed to create session",
                trace_id=str(uuid4()),
            )

        session = session_resp.json()
        session_id = session["session_id"]
        trace_id = session["trace_id"]

        # Resolve context and permissions
        resolve_resp = await client.post(
            f"{{CRA_RUNTIME_URL}}/v1/carp/resolve",
            json={{
                "session_id": session_id,
                "goal": f"Execute {{capability}}",
                "atlas_id": "{atlas.manifest.id}",
                "capability": capability.replace("/", "."),
            }},
        )

        if resolve_resp.status_code != 200:
            return ActionResponse(
                success=False,
                error="Resolution failed",
                trace_id=trace_id,
            )

        resolution = resolve_resp.json()

        # Check if action is allowed
        allowed_actions = resolution.get("resolution", {{}}).get("allowed_actions", [])
        action_id = capability.replace("/", ".")

        matching_action = None
        for action in allowed_actions:
            if action["action_id"] == action_id:
                matching_action = action
                break

        if not matching_action:
            return ActionResponse(
                success=False,
                error=f"Action {{action_id}} not permitted",
                trace_id=trace_id,
            )

        # Execute the action
        exec_resp = await client.post(
            f"{{CRA_RUNTIME_URL}}/v1/carp/execute",
            json={{
                "session_id": session_id,
                "resolution_id": resolution["resolution"]["resolution_id"],
                "action_id": action_id,
                "parameters": request.parameters,
            }},
        )

        if exec_resp.status_code != 200:
            return ActionResponse(
                success=False,
                error="Execution failed",
                trace_id=trace_id,
            )

        result = exec_resp.json()
        return ActionResponse(
            success=result.get("status") == "completed",
            result=result.get("result"),
            error=result.get("error"),
            trace_id=trace_id,
        )


@app.get("/health")
async def health() -> dict[str, str]:
    """Health check endpoint."""
    return {{"status": "healthy"}}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
'''

    def _generate_setup_instructions(self, atlas: LoadedAtlas) -> str:
        """Generate setup instructions."""
        return f"""# Setting up {atlas.manifest.name} GPT Action

## Prerequisites

1. OpenAI account with GPT-4 access
2. CRA runtime running (locally or deployed)
3. Python 3.11+

## Steps

### 1. Deploy the Bridge Server

```bash
# Install dependencies
pip install -r requirements.txt

# Set environment variables
export CRA_RUNTIME_URL=http://localhost:8420
export API_KEY=your-secret-key

# Run the server
python server.py
```

### 2. Configure the GPT

1. Go to https://chat.openai.com/gpts/editor
2. Create a new GPT
3. Copy the contents of `instructions.md` into the Instructions field
4. Under "Actions", click "Create new action"
5. Import `openapi.json`
6. Configure authentication if needed

### 3. Test the Integration

1. Start a conversation with your GPT
2. Ask it to perform one of the available actions
3. Verify the action executes through CRA

## Files

- `openapi.json` - OpenAPI specification for GPT Actions
- `instructions.md` - GPT system instructions
- `privacy-policy.md` - Privacy policy template
- `server.py` - Bridge server code

## Troubleshooting

- Check CRA runtime logs for resolution failures
- Verify API key is correctly configured
- Ensure the bridge server is accessible from OpenAI
"""
