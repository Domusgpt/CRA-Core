"""Core domain models for CRA."""

from cra.core.carp import (
    CARPEnvelope,
    CARPRequest,
    CARPResponse,
    Principal,
    Session,
    Task,
    Environment,
    Preferences,
    Resolution,
    ContextBlock,
    AllowedAction,
    DenyRule,
    MergeRules,
    NextStep,
)
from cra.core.trace import (
    TraceEvent,
    TraceContext,
    Actor,
    Artifact,
    Severity,
    EventType,
)
from cra.core.policy import (
    PolicyEngine,
    PolicyDecision,
    PolicyEffect,
    PolicyContext,
    PolicyRule,
    ScopeRule,
    DenyPatternRule,
    RiskTierApprovalRule,
    RateLimitRule,
    RedactionRule,
)
from cra.core.action import (
    ActionExecution,
    ActionGrant,
    ExecuteActionRequest,
    ExecuteActionResponse,
    ExecutionStatus,
)
from cra.core.validation import (
    SchemaValidator,
    SchemaValidationError,
)
from cra.core.replay import (
    TraceReplayer,
    ReplayManifest,
    ReplayResult,
    ReplayDifference,
)

__all__ = [
    # CARP types
    "CARPEnvelope",
    "CARPRequest",
    "CARPResponse",
    "Principal",
    "Session",
    "Task",
    "Environment",
    "Preferences",
    "Resolution",
    "ContextBlock",
    "AllowedAction",
    "DenyRule",
    "MergeRules",
    "NextStep",
    # TRACE types
    "TraceEvent",
    "TraceContext",
    "Actor",
    "Artifact",
    "Severity",
    "EventType",
    # Policy types
    "PolicyEngine",
    "PolicyDecision",
    "PolicyEffect",
    "PolicyContext",
    "PolicyRule",
    "ScopeRule",
    "DenyPatternRule",
    "RiskTierApprovalRule",
    "RateLimitRule",
    "RedactionRule",
    # Action types
    "ActionExecution",
    "ActionGrant",
    "ExecuteActionRequest",
    "ExecuteActionResponse",
    "ExecutionStatus",
    # Validation types
    "SchemaValidator",
    "SchemaValidationError",
    # Replay types
    "TraceReplayer",
    "ReplayManifest",
    "ReplayResult",
    "ReplayDifference",
]
