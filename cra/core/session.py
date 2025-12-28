"""Session management types for CRA runtime."""

from datetime import datetime, timedelta
from enum import Enum
from uuid import UUID, uuid4

from pydantic import BaseModel, Field


class SessionState(str, Enum):
    """State of a session."""

    ACTIVE = "active"
    EXPIRED = "expired"
    ENDED = "ended"


class SessionConfig(BaseModel):
    """Configuration for session creation."""

    ttl_seconds: int = Field(default=3600, ge=60, le=86400)  # 1min to 24h
    max_resolutions: int | None = None
    max_actions: int | None = None


class SessionPrincipal(BaseModel):
    """Principal that owns a session."""

    type: str  # "user", "service", "agent"
    id: str


class SessionStats(BaseModel):
    """Statistics for a session."""

    resolutions: int = 0
    actions_executed: int = 0
    actions_failed: int = 0
    total_events: int = 0


class Session(BaseModel):
    """A CRA session.

    Sessions track the lifecycle of an agent's interaction with the CRA runtime.
    Each session has a root trace_id for correlation.
    """

    session_id: UUID = Field(default_factory=uuid4)
    trace_id: UUID = Field(default_factory=uuid4)  # Root trace for this session
    principal: SessionPrincipal
    scopes: list[str] = Field(default_factory=list)
    state: SessionState = SessionState.ACTIVE
    created_at: datetime = Field(default_factory=datetime.utcnow)
    expires_at: datetime
    ended_at: datetime | None = None
    stats: SessionStats = Field(default_factory=SessionStats)
    config: SessionConfig = Field(default_factory=SessionConfig)

    @classmethod
    def create(
        cls,
        principal: SessionPrincipal,
        scopes: list[str] | None = None,
        config: SessionConfig | None = None,
    ) -> "Session":
        """Create a new session."""
        config = config or SessionConfig()
        now = datetime.utcnow()
        return cls(
            principal=principal,
            scopes=scopes or [],
            config=config,
            created_at=now,
            expires_at=now + timedelta(seconds=config.ttl_seconds),
        )

    @property
    def is_active(self) -> bool:
        """Check if session is still active."""
        if self.state != SessionState.ACTIVE:
            return False
        return datetime.utcnow() < self.expires_at

    @property
    def duration_seconds(self) -> float:
        """Get session duration in seconds."""
        end = self.ended_at or datetime.utcnow()
        return (end - self.created_at).total_seconds()

    def end(self) -> None:
        """End the session."""
        self.state = SessionState.ENDED
        self.ended_at = datetime.utcnow()

    def expire(self) -> None:
        """Mark session as expired."""
        self.state = SessionState.EXPIRED
        self.ended_at = datetime.utcnow()


class CreateSessionRequest(BaseModel):
    """Request to create a new session."""

    principal: SessionPrincipal
    scopes: list[str] = Field(default_factory=list)
    ttl_seconds: int = Field(default=3600, ge=60, le=86400)


class CreateSessionResponse(BaseModel):
    """Response after creating a session."""

    session_id: UUID
    trace_id: UUID
    expires_at: datetime


class EndSessionResponse(BaseModel):
    """Response after ending a session."""

    session_id: UUID
    ended_at: datetime
    trace_summary: SessionStats
