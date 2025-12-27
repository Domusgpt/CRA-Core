"""Tracer service - TRACE event emission.

Principle: If it wasn't emitted by the runtime, it didn't happen.

The Tracer is the sole authority for TRACE event emission.
All events are append-only and immutable.
"""

import asyncio
from collections import defaultdict
from datetime import datetime
from typing import AsyncIterator
from uuid import UUID, uuid4

from cra.core.trace import (
    Actor,
    ActorType,
    AtlasRef,
    EventType,
    Severity,
    TraceContext,
    TraceEvent,
)


class Tracer:
    """TRACE event emission service.

    This service is responsible for:
    - Emitting TRACE events
    - Storing events (in-memory for now, pluggable storage later)
    - Streaming events to subscribers
    """

    def __init__(self) -> None:
        """Initialize the tracer."""
        # In-memory storage: trace_id -> list of events
        self._events: dict[UUID, list[TraceEvent]] = defaultdict(list)
        # Subscribers for streaming: trace_id -> list of queues
        self._subscribers: dict[UUID, list[asyncio.Queue[TraceEvent]]] = defaultdict(list)
        # Lock for thread-safe operations
        self._lock = asyncio.Lock()

    async def emit(
        self,
        event_type: EventType,
        trace_id: UUID,
        session_id: UUID,
        payload: dict | None = None,
        *,
        span_id: UUID | None = None,
        parent_span_id: UUID | None = None,
        atlas: AtlasRef | None = None,
        actor_type: ActorType = ActorType.RUNTIME,
        actor_id: str = "cra-runtime",
        severity: Severity = Severity.INFO,
        artifacts: list | None = None,
    ) -> TraceEvent:
        """Emit a TRACE event.

        This is the ONLY way events should be created. The runtime is authoritative.

        Args:
            event_type: The type of event
            trace_id: Root trace ID
            session_id: Session that owns this trace
            payload: Event-specific payload
            span_id: Unique span ID (generated if not provided)
            parent_span_id: Parent span for correlation
            atlas: Atlas reference if applicable
            actor_type: Type of actor emitting the event
            actor_id: ID of the actor
            severity: Event severity level
            artifacts: Referenced artifacts

        Returns:
            The emitted TraceEvent
        """
        event = TraceEvent(
            event_type=event_type,
            time=datetime.utcnow(),
            trace=TraceContext(
                trace_id=trace_id,
                span_id=span_id or uuid4(),
                parent_span_id=parent_span_id,
            ),
            session_id=session_id,
            atlas=atlas,
            actor=Actor(type=actor_type, id=actor_id),
            severity=severity,
            payload=payload or {},
            artifacts=artifacts or [],
        )

        async with self._lock:
            # Append to storage (immutable - never modify)
            self._events[trace_id].append(event)

            # Notify subscribers
            for queue in self._subscribers[trace_id]:
                try:
                    queue.put_nowait(event)
                except asyncio.QueueFull:
                    # Drop events if subscriber can't keep up
                    pass

        return event

    async def get_events(
        self,
        trace_id: UUID,
        *,
        severity: Severity | None = None,
        event_type_prefix: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> tuple[list[TraceEvent], int]:
        """Get events for a trace.

        Args:
            trace_id: The trace to query
            severity: Filter by severity (optional)
            event_type_prefix: Filter by event type prefix (optional)
            limit: Maximum events to return
            offset: Offset for pagination

        Returns:
            Tuple of (events, total_count)
        """
        async with self._lock:
            events = self._events.get(trace_id, [])

        # Apply filters
        filtered = events
        if severity:
            filtered = [e for e in filtered if e.severity == severity]
        if event_type_prefix:
            filtered = [e for e in filtered if e.event_type.value.startswith(event_type_prefix)]

        total = len(filtered)
        paginated = filtered[offset : offset + limit]

        return paginated, total

    async def subscribe(self, trace_id: UUID) -> AsyncIterator[TraceEvent]:
        """Subscribe to events for a trace.

        Yields events as they are emitted. Used for SSE streaming.

        Args:
            trace_id: The trace to subscribe to

        Yields:
            TraceEvent as they are emitted
        """
        queue: asyncio.Queue[TraceEvent] = asyncio.Queue(maxsize=1000)

        async with self._lock:
            self._subscribers[trace_id].append(queue)

        try:
            while True:
                event = await queue.get()
                yield event
        finally:
            async with self._lock:
                self._subscribers[trace_id].remove(queue)

    async def get_all_events_for_session(self, session_id: UUID) -> list[TraceEvent]:
        """Get all events for a session across all traces.

        Args:
            session_id: The session ID

        Returns:
            List of all events for the session
        """
        async with self._lock:
            all_events = []
            for events in self._events.values():
                all_events.extend(e for e in events if e.session_id == session_id)
            return sorted(all_events, key=lambda e: e.time)

    def event_to_jsonl(self, event: TraceEvent) -> str:
        """Convert event to JSONL format for streaming.

        Args:
            event: The event to convert

        Returns:
            JSONL string (no trailing newline)
        """
        return event.model_dump_json()


# Global tracer instance (singleton for the runtime)
_tracer: Tracer | None = None


def get_tracer() -> Tracer:
    """Get the global tracer instance."""
    global _tracer
    if _tracer is None:
        _tracer = Tracer()
    return _tracer
