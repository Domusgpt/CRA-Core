"""Base storage interfaces for CRA Runtime."""

from abc import ABC, abstractmethod
from datetime import datetime
from typing import Any, AsyncIterator
from uuid import UUID

from cra.core.trace import TraceEvent
from cra.core.session import Session


class TraceStore(ABC):
    """Abstract base class for trace storage."""

    @abstractmethod
    async def append(self, event: TraceEvent) -> None:
        """Append a trace event.

        Args:
            event: The event to append
        """
        pass

    @abstractmethod
    async def get_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[TraceEvent]:
        """Get events for a trace.

        Args:
            trace_id: The trace ID
            event_type: Optional event type filter
            severity: Optional severity filter
            limit: Maximum events to return
            offset: Pagination offset

        Returns:
            List of trace events
        """
        pass

    @abstractmethod
    async def stream_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
    ) -> AsyncIterator[TraceEvent]:
        """Stream events for a trace.

        Args:
            trace_id: The trace ID
            event_type: Optional event type filter
            severity: Optional severity filter

        Yields:
            Trace events as they arrive
        """
        pass

    @abstractmethod
    async def get_event_count(self, trace_id: UUID) -> int:
        """Get the number of events for a trace.

        Args:
            trace_id: The trace ID

        Returns:
            Number of events
        """
        pass

    @abstractmethod
    async def delete_trace(self, trace_id: UUID) -> bool:
        """Delete all events for a trace.

        Args:
            trace_id: The trace ID

        Returns:
            True if deleted, False if not found
        """
        pass

    @abstractmethod
    async def get_traces(
        self,
        session_id: UUID | None = None,
        start_time: datetime | None = None,
        end_time: datetime | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        """Get trace summaries.

        Args:
            session_id: Optional session ID filter
            start_time: Optional start time filter
            end_time: Optional end time filter
            limit: Maximum traces to return
            offset: Pagination offset

        Returns:
            List of trace summaries
        """
        pass


class SessionStore(ABC):
    """Abstract base class for session storage."""

    @abstractmethod
    async def create(self, session: Session) -> Session:
        """Create a new session.

        Args:
            session: The session to create

        Returns:
            Created session
        """
        pass

    @abstractmethod
    async def get(self, session_id: UUID) -> Session | None:
        """Get a session by ID.

        Args:
            session_id: The session ID

        Returns:
            Session if found, None otherwise
        """
        pass

    @abstractmethod
    async def update(self, session: Session) -> Session:
        """Update a session.

        Args:
            session: The session to update

        Returns:
            Updated session
        """
        pass

    @abstractmethod
    async def delete(self, session_id: UUID) -> bool:
        """Delete a session.

        Args:
            session_id: The session ID

        Returns:
            True if deleted, False if not found
        """
        pass

    @abstractmethod
    async def list_active(
        self,
        principal_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Session]:
        """List active sessions.

        Args:
            principal_id: Optional principal ID filter
            limit: Maximum sessions to return
            offset: Pagination offset

        Returns:
            List of active sessions
        """
        pass

    @abstractmethod
    async def cleanup_expired(self) -> int:
        """Clean up expired sessions.

        Returns:
            Number of sessions cleaned up
        """
        pass
