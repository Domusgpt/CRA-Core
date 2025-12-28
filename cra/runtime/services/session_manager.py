"""Session manager service.

Manages the lifecycle of CRA sessions.
"""

import asyncio
from datetime import datetime
from uuid import UUID

from cra.core.session import (
    CreateSessionRequest,
    CreateSessionResponse,
    EndSessionResponse,
    Session,
    SessionConfig,
    SessionPrincipal,
    SessionState,
)
from cra.core.trace import EventType, Severity
from cra.runtime.services.tracer import Tracer


class SessionNotFoundError(Exception):
    """Session not found."""

    pass


class SessionExpiredError(Exception):
    """Session has expired."""

    pass


class SessionManager:
    """Manages CRA sessions.

    Sessions track the lifecycle of an agent's interaction with the runtime.
    Each session has a root trace_id for correlating all events.
    """

    def __init__(self, tracer: Tracer) -> None:
        """Initialize the session manager.

        Args:
            tracer: The tracer service for event emission
        """
        self._tracer = tracer
        self._sessions: dict[UUID, Session] = {}
        self._lock = asyncio.Lock()

    async def create_session(self, request: CreateSessionRequest) -> CreateSessionResponse:
        """Create a new session.

        Args:
            request: Session creation request

        Returns:
            Session creation response with session_id and trace_id
        """
        config = SessionConfig(ttl_seconds=request.ttl_seconds)
        session = Session.create(
            principal=request.principal,
            scopes=request.scopes,
            config=config,
        )

        async with self._lock:
            self._sessions[session.session_id] = session

        # Emit session started event
        await self._tracer.emit(
            event_type=EventType.SESSION_STARTED,
            trace_id=session.trace_id,
            session_id=session.session_id,
            payload={
                "principal_type": session.principal.type,
                "principal_id": session.principal.id,
                "scopes": session.scopes,
                "ttl_seconds": config.ttl_seconds,
            },
        )

        return CreateSessionResponse(
            session_id=session.session_id,
            trace_id=session.trace_id,
            expires_at=session.expires_at,
        )

    async def end_session(self, session_id: UUID) -> EndSessionResponse:
        """End a session.

        Args:
            session_id: The session to end

        Returns:
            Session end response with summary stats

        Raises:
            SessionNotFoundError: If session doesn't exist
        """
        async with self._lock:
            session = self._sessions.get(session_id)
            if session is None:
                raise SessionNotFoundError(f"Session {session_id} not found")

            session.end()

        # Get all events for this session to compute stats
        events = await self._tracer.get_all_events_for_session(session_id)
        session.stats.total_events = len(events)

        # Emit session ended event
        await self._tracer.emit(
            event_type=EventType.SESSION_ENDED,
            trace_id=session.trace_id,
            session_id=session.session_id,
            payload={
                "duration_seconds": session.duration_seconds,
                "total_events": session.stats.total_events,
                "resolutions": session.stats.resolutions,
                "actions_executed": session.stats.actions_executed,
            },
        )

        return EndSessionResponse(
            session_id=session.session_id,
            ended_at=session.ended_at,  # type: ignore
            trace_summary=session.stats,
        )

    async def get_session(self, session_id: UUID) -> Session:
        """Get a session by ID.

        Args:
            session_id: The session ID

        Returns:
            The session

        Raises:
            SessionNotFoundError: If session doesn't exist
            SessionExpiredError: If session has expired
        """
        async with self._lock:
            session = self._sessions.get(session_id)
            if session is None:
                raise SessionNotFoundError(f"Session {session_id} not found")

            # Check expiration
            if session.state == SessionState.ACTIVE and not session.is_active:
                session.expire()
                # Emit expiration event
                await self._tracer.emit(
                    event_type=EventType.SESSION_ENDED,
                    trace_id=session.trace_id,
                    session_id=session.session_id,
                    severity=Severity.WARN,
                    payload={
                        "reason": "expired",
                        "duration_seconds": session.duration_seconds,
                    },
                )
                raise SessionExpiredError(f"Session {session_id} has expired")

            if session.state != SessionState.ACTIVE:
                raise SessionExpiredError(f"Session {session_id} is {session.state.value}")

            return session

    async def increment_resolution_count(self, session_id: UUID) -> None:
        """Increment the resolution count for a session.

        Args:
            session_id: The session ID
        """
        async with self._lock:
            session = self._sessions.get(session_id)
            if session:
                session.stats.resolutions += 1

    async def increment_action_count(self, session_id: UUID, failed: bool = False) -> None:
        """Increment the action count for a session.

        Args:
            session_id: The session ID
            failed: Whether the action failed
        """
        async with self._lock:
            session = self._sessions.get(session_id)
            if session:
                if failed:
                    session.stats.actions_failed += 1
                else:
                    session.stats.actions_executed += 1

    async def list_active_sessions(self) -> list[Session]:
        """List all active sessions.

        Returns:
            List of active sessions
        """
        async with self._lock:
            return [s for s in self._sessions.values() if s.is_active]


# Global session manager instance
_session_manager: SessionManager | None = None


def get_session_manager(tracer: Tracer) -> SessionManager:
    """Get the global session manager instance."""
    global _session_manager
    if _session_manager is None:
        _session_manager = SessionManager(tracer)
    return _session_manager
