"""In-memory storage implementations for development."""

import asyncio
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any, AsyncIterator
from uuid import UUID

from cra.core.trace import TraceEvent
from cra.core.session import Session
from cra.runtime.storage.base import TraceStore, SessionStore


class InMemoryTraceStore(TraceStore):
    """In-memory trace storage for development."""

    def __init__(self):
        self._events: dict[UUID, list[TraceEvent]] = defaultdict(list)
        self._subscribers: dict[UUID, list[asyncio.Queue]] = defaultdict(list)

    async def append(self, event: TraceEvent) -> None:
        """Append a trace event."""
        self._events[event.trace.trace_id].append(event)

        # Notify subscribers
        for queue in self._subscribers.get(event.trace.trace_id, []):
            await queue.put(event)

    async def get_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[TraceEvent]:
        """Get events for a trace."""
        events = self._events.get(trace_id, [])

        # Apply filters
        if event_type:
            events = [e for e in events if e.event_type.startswith(event_type)]
        if severity:
            events = [e for e in events if e.severity.value == severity]

        # Apply pagination
        return events[offset : offset + limit]

    async def stream_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
    ) -> AsyncIterator[TraceEvent]:
        """Stream events for a trace."""
        # First yield existing events
        for event in self._events.get(trace_id, []):
            if event_type and not event.event_type.startswith(event_type):
                continue
            if severity and event.severity.value != severity:
                continue
            yield event

        # Then subscribe to new events
        queue: asyncio.Queue[TraceEvent] = asyncio.Queue()
        self._subscribers[trace_id].append(queue)

        try:
            while True:
                event = await queue.get()
                if event_type and not event.event_type.startswith(event_type):
                    continue
                if severity and event.severity.value != severity:
                    continue
                yield event
        finally:
            self._subscribers[trace_id].remove(queue)

    async def get_event_count(self, trace_id: UUID) -> int:
        """Get event count for a trace."""
        return len(self._events.get(trace_id, []))

    async def delete_trace(self, trace_id: UUID) -> bool:
        """Delete all events for a trace."""
        if trace_id in self._events:
            del self._events[trace_id]
            return True
        return False

    async def get_traces(
        self,
        session_id: UUID | None = None,
        start_time: datetime | None = None,
        end_time: datetime | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        """Get trace summaries."""
        summaries = []

        for trace_id, events in self._events.items():
            if not events:
                continue

            first_event = events[0]
            last_event = events[-1]

            # Apply filters
            if session_id and first_event.session_id != session_id:
                continue
            if start_time and first_event.time < start_time:
                continue
            if end_time and first_event.time > end_time:
                continue

            summaries.append({
                "trace_id": str(trace_id),
                "session_id": str(first_event.session_id),
                "event_count": len(events),
                "first_event_time": first_event.time.isoformat(),
                "last_event_time": last_event.time.isoformat(),
            })

        # Sort by first event time descending
        summaries.sort(key=lambda x: x["first_event_time"], reverse=True)

        return summaries[offset : offset + limit]


class InMemorySessionStore(SessionStore):
    """In-memory session storage for development."""

    def __init__(self):
        self._sessions: dict[UUID, Session] = {}

    async def create(self, session: Session) -> Session:
        """Create a new session."""
        self._sessions[session.session_id] = session
        return session

    async def get(self, session_id: UUID) -> Session | None:
        """Get a session by ID."""
        return self._sessions.get(session_id)

    async def update(self, session: Session) -> Session:
        """Update a session."""
        self._sessions[session.session_id] = session
        return session

    async def delete(self, session_id: UUID) -> bool:
        """Delete a session."""
        if session_id in self._sessions:
            del self._sessions[session_id]
            return True
        return False

    async def list_active(
        self,
        principal_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Session]:
        """List active sessions."""
        now = datetime.now(timezone.utc)
        sessions = [
            s for s in self._sessions.values()
            if s.expires_at > now and s.ended_at is None
        ]

        if principal_id:
            sessions = [s for s in sessions if s.principal.id == principal_id]

        # Sort by created_at descending
        sessions.sort(key=lambda x: x.created_at, reverse=True)

        return sessions[offset : offset + limit]

    async def cleanup_expired(self) -> int:
        """Clean up expired sessions."""
        now = datetime.now(timezone.utc)
        expired = [
            sid for sid, s in self._sessions.items()
            if s.expires_at <= now
        ]

        for sid in expired:
            del self._sessions[sid]

        return len(expired)
