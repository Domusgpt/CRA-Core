"""CARP/1.0 - Context & Action Resolution Protocol types.

CARP defines a deterministic contract between:
- an acting agent (Requester), and
- a context authority (CRA Resolver)

CARP answers what context is allowed and what actions may occur.
"""

from datetime import datetime
from enum import Enum
from typing import Any, Literal
from uuid import UUID

from pydantic import BaseModel, Field

from cra.version import CARP_VERSION


class PrincipalType(str, Enum):
    """Type of principal making a request."""

    USER = "user"
    SERVICE = "service"
    AGENT = "agent"


class Principal(BaseModel):
    """Identity making a CARP request."""

    type: PrincipalType
    id: str = Field(..., min_length=1)


class Session(BaseModel):
    """Session context for a CARP request."""

    session_id: UUID
    principal: Principal
    scopes: list[str] = Field(default_factory=list)
    expires_at: datetime | None = None


class AtlasRef(BaseModel):
    """Reference to an Atlas being used."""

    id: str = Field(..., min_length=1)
    version: str = Field(..., pattern=r"^\d+\.\d+\.\d+")
    capability: str | None = None


class TraceContext(BaseModel):
    """Distributed trace context."""

    trace_id: UUID
    span_id: UUID
    parent_span_id: UUID | None = None


# === Request Types ===


class RiskTier(str, Enum):
    """Risk classification for a task."""

    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"


class InputType(str, Enum):
    """Type of task input."""

    TEXT = "text"
    JSON = "json"
    URI = "uri"
    FILE_REF = "file_ref"


class TaskInput(BaseModel):
    """Input to a task."""

    name: str
    type: InputType
    value: Any


class Task(BaseModel):
    """Task to be resolved by CARP."""

    goal: str = Field(..., min_length=1)
    inputs: list[TaskInput] = Field(default_factory=list)
    constraints: list[str] = Field(default_factory=list)
    target_platforms: list[str] = Field(
        default_factory=lambda: ["openai.tools", "anthropic.skills", "google.adk", "mcp"]
    )
    risk_tier: RiskTier = RiskTier.MEDIUM


class NetworkPolicy(str, Enum):
    """Network access policy."""

    OFFLINE = "offline"
    RESTRICTED = "restricted"
    OPEN = "open"


class Environment(BaseModel):
    """Environment context for resolution."""

    project_root: str | None = None
    os: str | None = None
    cli_capabilities: list[str] = Field(default_factory=lambda: ["bash", "python", "git"])
    network_policy: NetworkPolicy = NetworkPolicy.OPEN


class Verbosity(str, Enum):
    """Output verbosity level."""

    COMPACT = "compact"
    STANDARD = "standard"
    EXTENDED = "extended"


class Explainability(str, Enum):
    """Explainability level for responses."""

    MINIMAL = "minimal"
    STANDARD = "standard"
    DEEP = "deep"


class Preferences(BaseModel):
    """User preferences for resolution."""

    verbosity: Verbosity = Verbosity.STANDARD
    format: list[str] = Field(default_factory=lambda: ["json", "markdown"])
    explainability: Explainability = Explainability.STANDARD


class ResolveRequestPayload(BaseModel):
    """Payload for a CARP resolve request."""

    operation: Literal["resolve"] = "resolve"
    task: Task
    environment: Environment = Field(default_factory=Environment)
    preferences: Preferences = Field(default_factory=Preferences)


# === Response Types ===


class ContentType(str, Enum):
    """Content type for context blocks."""

    MARKDOWN = "text/markdown"
    JSON = "application/json"
    PLAIN = "text/plain"
    PNG = "image/png"


class Redaction(BaseModel):
    """Redaction applied to a context block."""

    field: str
    reason: str


class EvidenceType(str, Enum):
    """Type of source evidence."""

    DOC = "doc"
    API = "api"
    POLICY = "policy"
    TEST = "test"


class SourceEvidence(BaseModel):
    """Evidence backing a context block."""

    type: EvidenceType
    ref: str
    hash: str = Field(..., pattern=r"^[a-f0-9]{64}$")  # SHA256


class ContextBlock(BaseModel):
    """A block of context returned by CARP resolution."""

    block_id: str
    purpose: str
    ttl_seconds: int = Field(default=3600, ge=0)
    content_type: ContentType = ContentType.MARKDOWN
    content: str | dict[str, Any]
    redactions: list[Redaction] = Field(default_factory=list)
    source_evidence: list[SourceEvidence] = Field(default_factory=list)


class ActionKind(str, Enum):
    """Kind of action."""

    TOOL_CALL = "tool_call"
    MCP_CALL = "mcp_call"
    CLI_COMMAND = "cli_command"
    AGENT_TOOL = "agent_tool"


class ConstraintType(str, Enum):
    """Type of constraint on an action."""

    RATE_LIMIT = "rate_limit"
    SCOPE = "scope"
    APPROVAL = "approval"
    SANDBOX = "sandbox"


class ActionConstraint(BaseModel):
    """Constraint on an allowed action."""

    type: ConstraintType
    value: str | int | dict[str, Any]


class AllowedAction(BaseModel):
    """An action permitted by CARP resolution."""

    action_id: str
    kind: ActionKind
    adapter: str
    description: str = ""
    schema: dict[str, Any] = Field(default_factory=dict, alias="json_schema")
    constraints: list[ActionConstraint] = Field(default_factory=list)
    requires_approval: bool = False
    timeout_ms: int = Field(default=30000, ge=0)


class DenyRule(BaseModel):
    """A pattern to deny."""

    pattern: str
    reason: str


class ConflictResolution(str, Enum):
    """How to resolve conflicts in merge."""

    FAIL = "fail"
    LAST_WRITE_WINS = "last_write_wins"
    PRIORITY = "priority"


class MergeRules(BaseModel):
    """Rules for merging multiple resolutions."""

    conflict: ConflictResolution = ConflictResolution.FAIL


class NextStep(BaseModel):
    """Suggested next step after resolution."""

    step: str
    expected_artifacts: list[str] = Field(default_factory=list)


class Resolution(BaseModel):
    """The resolution result from CARP."""

    resolution_id: UUID
    confidence: float = Field(..., ge=0.0, le=1.0)
    context_blocks: list[ContextBlock] = Field(default_factory=list)
    allowed_actions: list[AllowedAction] = Field(default_factory=list)
    denylist: list[DenyRule] = Field(default_factory=list)
    merge_rules: MergeRules = Field(default_factory=MergeRules)
    next_steps: list[NextStep] = Field(default_factory=list)


class ResolveResponsePayload(BaseModel):
    """Payload for a CARP resolve response."""

    operation: Literal["resolve"] = "resolve"
    resolution: Resolution


# === Envelope Types ===


class CARPEnvelope(BaseModel):
    """Base envelope for all CARP messages."""

    carp_version: str = CARP_VERSION
    type: Literal["carp.request", "carp.response"]
    id: UUID
    time: datetime
    session: Session
    atlas: AtlasRef | None = None
    payload: dict[str, Any]
    trace: TraceContext


class CARPRequest(CARPEnvelope):
    """A CARP request envelope."""

    type: Literal["carp.request"] = "carp.request"
    payload: ResolveRequestPayload  # type: ignore[assignment]


class CARPResponse(CARPEnvelope):
    """A CARP response envelope."""

    type: Literal["carp.response"] = "carp.response"
    payload: ResolveResponsePayload  # type: ignore[assignment]
