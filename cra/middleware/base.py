"""Base CRA middleware for agent frameworks.

Provides common functionality for integrating CRA governance
with any agent framework.
"""

import os
from typing import Any
from uuid import UUID

import httpx
from pydantic import BaseModel, Field


class CRASession(BaseModel):
    """Active CRA session."""

    session_id: UUID
    trace_id: UUID
    expires_at: str


class CRAResolution(BaseModel):
    """CARP resolution result."""

    resolution_id: UUID
    confidence: float
    context_blocks: list[dict[str, Any]] = Field(default_factory=list)
    allowed_actions: list[dict[str, Any]] = Field(default_factory=list)
    denylist: list[dict[str, Any]] = Field(default_factory=list)


class CRAMiddleware:
    """Base middleware for CRA integration.

    Provides core functionality for:
    - Session management
    - Context resolution
    - Action execution
    - Trace access

    Usage:
        middleware = CRAMiddleware(runtime_url="http://localhost:8420")

        # Create session and resolve context
        resolution = middleware.resolve(
            goal="Deploy to staging",
            atlas_id="com.example.devops",
            capability="deploy.staging",
        )

        # Execute an action
        result = middleware.execute(
            action_id="deploy.staging",
            parameters={"service": "api", "version": "1.2.3"},
        )
    """

    def __init__(
        self,
        runtime_url: str | None = None,
        api_key: str | None = None,
        jwt_token: str | None = None,
        principal_id: str = "middleware-agent",
        principal_type: str = "agent",
        default_scopes: list[str] | None = None,
        timeout: float = 30.0,
    ):
        self.runtime_url = runtime_url or os.getenv(
            "CRA_RUNTIME_URL", "http://localhost:8420"
        )
        self.api_key = api_key or os.getenv("CRA_API_KEY")
        self.jwt_token = jwt_token or os.getenv("CRA_JWT_TOKEN")
        self.principal_id = principal_id
        self.principal_type = principal_type
        self.default_scopes = default_scopes or []
        self.timeout = timeout

        self._session: CRASession | None = None
        self._resolution: CRAResolution | None = None
        self._client = httpx.Client(timeout=timeout)

    def _get_headers(self) -> dict[str, str]:
        """Get authentication headers."""
        headers = {"Content-Type": "application/json"}
        if self.jwt_token:
            headers["Authorization"] = f"Bearer {self.jwt_token}"
        elif self.api_key:
            headers["X-API-Key"] = self.api_key
        return headers

    def create_session(
        self,
        scopes: list[str] | None = None,
        ttl_seconds: int = 3600,
    ) -> CRASession:
        """Create a new CRA session.

        Args:
            scopes: List of scopes for the session
            ttl_seconds: Session TTL in seconds

        Returns:
            Created session
        """
        response = self._client.post(
            f"{self.runtime_url}/v1/sessions",
            headers=self._get_headers(),
            json={
                "principal": {
                    "type": self.principal_type,
                    "id": self.principal_id,
                },
                "scopes": scopes or self.default_scopes,
                "ttl_seconds": ttl_seconds,
            },
        )
        response.raise_for_status()
        data = response.json()

        self._session = CRASession(
            session_id=UUID(data["session_id"]),
            trace_id=UUID(data["trace_id"]),
            expires_at=data["expires_at"],
        )
        return self._session

    def ensure_session(self, scopes: list[str] | None = None) -> CRASession:
        """Ensure we have an active session.

        Args:
            scopes: Optional scopes to add

        Returns:
            Active session
        """
        if not self._session:
            return self.create_session(scopes)
        return self._session

    def resolve(
        self,
        goal: str,
        atlas_id: str | None = None,
        capability: str | None = None,
        risk_tier: str = "medium",
        context: dict[str, Any] | None = None,
    ) -> CRAResolution:
        """Resolve context and permissions.

        Args:
            goal: The agent's goal
            atlas_id: Optional Atlas ID
            capability: Optional capability filter
            risk_tier: Risk tier for the operation
            context: Additional context

        Returns:
            Resolution with context and allowed actions
        """
        self.ensure_session()

        response = self._client.post(
            f"{self.runtime_url}/v1/carp/resolve",
            headers=self._get_headers(),
            json={
                "session_id": str(self._session.session_id),
                "goal": goal,
                "atlas_id": atlas_id,
                "capability": capability,
                "risk_tier": risk_tier,
                "context": context or {},
            },
        )
        response.raise_for_status()
        data = response.json()

        resolution_data = data.get("resolution", data)
        self._resolution = CRAResolution(
            resolution_id=UUID(resolution_data["resolution_id"]),
            confidence=resolution_data["confidence"],
            context_blocks=resolution_data.get("context_blocks", []),
            allowed_actions=resolution_data.get("allowed_actions", []),
            denylist=resolution_data.get("denylist", []),
        )
        return self._resolution

    def execute(
        self,
        action_id: str,
        parameters: dict[str, Any] | None = None,
        resolution_id: UUID | None = None,
    ) -> dict[str, Any]:
        """Execute a CRA-governed action.

        Args:
            action_id: The action to execute
            parameters: Action parameters
            resolution_id: Optional resolution ID (uses current if not provided)

        Returns:
            Execution result
        """
        self.ensure_session()

        if resolution_id is None and self._resolution:
            resolution_id = self._resolution.resolution_id
        elif resolution_id is None:
            raise ValueError("No resolution available. Call resolve() first.")

        response = self._client.post(
            f"{self.runtime_url}/v1/carp/execute",
            headers=self._get_headers(),
            json={
                "session_id": str(self._session.session_id),
                "resolution_id": str(resolution_id),
                "action_id": action_id,
                "parameters": parameters or {},
            },
        )
        response.raise_for_status()
        return response.json()

    def resolve_and_inject(
        self,
        goal: str,
        platform: str = "openai",
        atlas_id: str | None = None,
        capability: str | None = None,
    ) -> dict[str, Any]:
        """Resolve context and return platform-specific format.

        Args:
            goal: The agent's goal
            platform: Target platform (openai, anthropic, etc.)
            atlas_id: Optional Atlas ID
            capability: Optional capability filter

        Returns:
            Platform-specific tool/context format
        """
        self.resolve(goal, atlas_id, capability)

        # Use the emit endpoint to get platform-specific format
        if atlas_id:
            response = self._client.get(
                f"{self.runtime_url}/v1/atlases/{atlas_id}/emit/{platform}",
                headers=self._get_headers(),
            )
            response.raise_for_status()
            return response.json().get("output", {})

        # Build from resolution directly
        from cra.adapters import get_adapter
        from cra.core.carp import Resolution, AllowedAction, ContextBlock, MergeRules

        # Convert to proper types
        allowed_actions = [
            AllowedAction.model_validate(a) for a in self._resolution.allowed_actions
        ]

        adapter = get_adapter(platform)
        output = adapter.emit_tools(allowed_actions)
        return output.to_dict()

    def get_trace_id(self) -> UUID | None:
        """Get the current trace ID."""
        return self._session.trace_id if self._session else None

    def get_session_id(self) -> UUID | None:
        """Get the current session ID."""
        return self._session.session_id if self._session else None

    def end_session(self) -> None:
        """End the current session."""
        if self._session:
            try:
                self._client.post(
                    f"{self.runtime_url}/v1/sessions/{self._session.session_id}/end",
                    headers=self._get_headers(),
                )
            except Exception:
                pass
            self._session = None
            self._resolution = None

    def is_action_allowed(self, action_id: str) -> bool:
        """Check if an action is allowed in current resolution.

        Args:
            action_id: The action to check

        Returns:
            True if allowed
        """
        if not self._resolution:
            return False

        return any(
            a.get("action_id") == action_id
            for a in self._resolution.allowed_actions
        )

    def is_pattern_denied(self, pattern: str) -> tuple[bool, str | None]:
        """Check if a pattern is in the denylist.

        Args:
            pattern: Pattern to check

        Returns:
            Tuple of (is_denied, reason)
        """
        if not self._resolution:
            return False, None

        import fnmatch
        for rule in self._resolution.denylist:
            if fnmatch.fnmatch(pattern, rule.get("pattern", "")):
                return True, rule.get("reason")

        return False, None

    def __enter__(self) -> "CRAMiddleware":
        """Context manager entry."""
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Context manager exit - cleanup session."""
        self.end_session()
        self._client.close()

    def close(self) -> None:
        """Close the middleware and cleanup resources."""
        self.end_session()
        self._client.close()
