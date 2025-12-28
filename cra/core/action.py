"""Action execution types for CRA.

Provides types for executing granted actions and tracking
their results in TRACE.
"""

from datetime import datetime
from enum import Enum
from typing import Any
from uuid import UUID, uuid4

from pydantic import BaseModel, Field


class ExecutionStatus(str, Enum):
    """Status of an action execution."""

    PENDING = "pending"
    APPROVED = "approved"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    REJECTED = "rejected"


class ExecuteActionRequest(BaseModel):
    """Request to execute a granted action."""

    session_id: UUID
    resolution_id: UUID
    action_id: str
    parameters: dict[str, Any] = Field(default_factory=dict)
    trace_id: UUID
    span_id: UUID = Field(default_factory=uuid4)
    parent_span_id: UUID | None = None


class ActionGrant(BaseModel):
    """A granted action with execution context."""

    grant_id: UUID = Field(default_factory=uuid4)
    resolution_id: UUID
    action_id: str
    kind: str
    adapter: str
    schema: dict[str, Any] = Field(default_factory=dict)
    constraints: list[dict[str, Any]] = Field(default_factory=list)
    requires_approval: bool = False
    approved: bool = False
    approved_by: str | None = None
    approved_at: datetime | None = None
    expires_at: datetime
    created_at: datetime = Field(default_factory=datetime.utcnow)


class ActionExecution(BaseModel):
    """Record of an action execution."""

    execution_id: UUID = Field(default_factory=uuid4)
    grant_id: UUID
    session_id: UUID
    action_id: str
    parameters: dict[str, Any] = Field(default_factory=dict)
    parameters_hash: str = ""  # SHA256 of parameters
    status: ExecutionStatus = ExecutionStatus.PENDING
    result: dict[str, Any] | None = None
    result_hash: str | None = None  # SHA256 of result
    error: dict[str, Any] | None = None
    started_at: datetime | None = None
    completed_at: datetime | None = None
    duration_ms: int | None = None
    trace_id: UUID
    span_id: UUID


class ExecuteActionResponse(BaseModel):
    """Response from action execution."""

    execution_id: UUID
    status: ExecutionStatus
    result: dict[str, Any] | None = None
    error: dict[str, Any] | None = None
    duration_ms: int | None = None
    trace: dict[str, Any] = Field(default_factory=dict)


class ApprovalRequest(BaseModel):
    """Request for action approval."""

    grant_id: UUID
    action_id: str
    reason: str
    risk_tier: str
    requested_by: str
    requested_at: datetime = Field(default_factory=datetime.utcnow)
    context: dict[str, Any] = Field(default_factory=dict)


class ApprovalResponse(BaseModel):
    """Response to an approval request."""

    grant_id: UUID
    approved: bool
    approved_by: str | None = None
    approved_at: datetime | None = None
    reason: str = ""
