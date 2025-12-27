"""TRACE/1.0 - Telemetry & Replay Artifact Contract types.

Principle: If it wasn't emitted by the runtime, it didn't happen.

TRACE is an append-only event stream emitted by the CRA runtime
that supports replay and diff for auditing and testing.
"""

from datetime import datetime
from enum import Enum
from typing import Any
from uuid import UUID

from pydantic import BaseModel, Field

from cra.version import TRACE_VERSION


class Severity(str, Enum):
    """Severity level for TRACE events."""

    DEBUG = "debug"
    INFO = "info"
    WARN = "warn"
    ERROR = "error"


class ActorType(str, Enum):
    """Type of actor that generated an event."""

    RUNTIME = "runtime"
    AGENT = "agent"
    USER = "user"
    TOOL = "tool"


class Actor(BaseModel):
    """Actor that generated a TRACE event."""

    type: ActorType
    id: str


class TraceContext(BaseModel):
    """Distributed trace context for correlation."""

    trace_id: UUID
    span_id: UUID
    parent_span_id: UUID | None = None


class AtlasRef(BaseModel):
    """Reference to the Atlas in use."""

    id: str
    version: str


class Artifact(BaseModel):
    """Artifact referenced by a TRACE event."""

    name: str
    uri: str
    sha256: str = Field(..., pattern=r"^[a-f0-9]{64}$")
    content_type: str


class EventType(str, Enum):
    """Mandatory TRACE event types.

    Event type naming convention: trace.<domain>.<action>
    """

    # Session events
    SESSION_STARTED = "trace.session.started"
    SESSION_ENDED = "trace.session.ended"

    # CARP events
    CARP_RESOLVE_REQUESTED = "trace.carp.resolve.requested"
    CARP_RESOLVE_RETURNED = "trace.carp.resolve.returned"
    CARP_POLICY_DENIED = "trace.carp.policy.denied"

    # Action events
    ACTION_GRANTED = "trace.action.granted"
    ACTION_INVOKED = "trace.action.invoked"
    ACTION_COMPLETED = "trace.action.completed"
    ACTION_FAILED = "trace.action.failed"

    # Artifact events
    ARTIFACT_CREATED = "trace.artifact.created"
    ARTIFACT_UPDATED = "trace.artifact.updated"
    ARTIFACT_REDACTED = "trace.artifact.redacted"

    # Error events
    RUNTIME_ERROR = "trace.runtime.error"
    ADAPTER_ERROR = "trace.adapter.error"
    VALIDATION_ERROR = "trace.validation.error"


class TraceEvent(BaseModel):
    """A single TRACE event.

    TRACE events are append-only and immutable once emitted.
    The runtime is the sole authority for event emission.
    """

    trace_version: str = TRACE_VERSION
    event_type: EventType
    time: datetime
    trace: TraceContext
    session_id: UUID
    atlas: AtlasRef | None = None
    actor: Actor
    severity: Severity = Severity.INFO
    payload: dict[str, Any] = Field(default_factory=dict)
    artifacts: list[Artifact] = Field(default_factory=list)

    class Config:
        """Pydantic configuration."""

        use_enum_values = True


# === Session Event Payloads ===


class SessionStartedPayload(BaseModel):
    """Payload for trace.session.started events."""

    principal_type: str
    principal_id: str
    scopes: list[str]
    ttl_seconds: int


class SessionEndedPayload(BaseModel):
    """Payload for trace.session.ended events."""

    duration_seconds: float
    total_events: int
    resolutions: int
    actions_executed: int


# === CARP Event Payloads ===


class CARPResolveRequestedPayload(BaseModel):
    """Payload for trace.carp.resolve.requested events."""

    goal: str
    risk_tier: str
    target_platforms: list[str]


class CARPResolveReturnedPayload(BaseModel):
    """Payload for trace.carp.resolve.returned events."""

    resolution_id: str
    confidence: float
    context_block_count: int
    allowed_action_count: int
    deny_rule_count: int


class CARPPolicyDeniedPayload(BaseModel):
    """Payload for trace.carp.policy.denied events."""

    reason: str
    policy_id: str
    denied_resource: str


# === Action Event Payloads ===


class ActionGrantedPayload(BaseModel):
    """Payload for trace.action.granted events."""

    action_id: str
    resolution_id: str
    requires_approval: bool


class ActionInvokedPayload(BaseModel):
    """Payload for trace.action.invoked events."""

    action_id: str
    execution_id: str
    parameters_hash: str  # SHA256 of parameters for audit


class ActionCompletedPayload(BaseModel):
    """Payload for trace.action.completed events."""

    action_id: str
    execution_id: str
    duration_ms: int
    result_hash: str  # SHA256 of result for audit


class ActionFailedPayload(BaseModel):
    """Payload for trace.action.failed events."""

    action_id: str
    execution_id: str
    error_type: str
    error_message: str
    duration_ms: int


# === Error Event Payloads ===


class RuntimeErrorPayload(BaseModel):
    """Payload for trace.runtime.error events."""

    error_type: str
    error_message: str
    stack_trace: str | None = None


class ValidationErrorPayload(BaseModel):
    """Payload for trace.validation.error events."""

    field: str
    expected: str
    received: str
    message: str


# === Replay Types ===


class NondeterminismRule(BaseModel):
    """Rule for handling nondeterminism in replay."""

    field: str  # JSONPath-like field selector
    rule: str  # "ignore", "normalize", "mask"


class ReplayManifest(BaseModel):
    """Manifest for replaying a trace."""

    manifest_version: str = "1.0"
    trace_id: UUID
    created_at: datetime
    description: str = ""
    artifacts: list[Artifact] = Field(default_factory=list)
    nondeterminism: list[NondeterminismRule] = Field(default_factory=list)
    expected_event_count: int = 0
