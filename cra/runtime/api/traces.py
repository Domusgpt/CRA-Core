"""TRACE query and streaming endpoints."""

import asyncio
from typing import AsyncIterator
from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, Query, status
from pydantic import BaseModel
from sse_starlette.sse import EventSourceResponse

from cra.core.trace import Severity, TraceEvent
from cra.runtime.services.tracer import Tracer
from cra.runtime.api.dependencies import get_tracer_dep

router = APIRouter(prefix="/v1/traces", tags=["traces"])


class TraceEventsResponse(BaseModel):
    """Response for trace events query."""

    trace_id: UUID
    events: list[TraceEvent]
    total_count: int
    has_more: bool


@router.get("/{trace_id}/events", response_model=TraceEventsResponse)
async def get_trace_events(
    trace_id: UUID,
    severity: Severity | None = Query(None, description="Filter by severity"),
    event_type: str | None = Query(None, description="Filter by event type prefix"),
    limit: int = Query(100, ge=1, le=1000, description="Maximum events to return"),
    offset: int = Query(0, ge=0, description="Offset for pagination"),
    tracer: Tracer = Depends(get_tracer_dep),
) -> TraceEventsResponse:
    """Get events for a trace.

    Returns paginated TRACE events for a given trace_id.
    Events can be filtered by severity and event type.
    """
    events, total = await tracer.get_events(
        trace_id,
        severity=severity,
        event_type_prefix=event_type,
        limit=limit,
        offset=offset,
    )

    return TraceEventsResponse(
        trace_id=trace_id,
        events=events,
        total_count=total,
        has_more=(offset + len(events)) < total,
    )


@router.get("/{trace_id}/stream")
async def stream_trace_events(
    trace_id: UUID,
    tracer: Tracer = Depends(get_tracer_dep),
) -> EventSourceResponse:
    """Stream trace events via Server-Sent Events (SSE).

    Subscribes to the trace and streams events as they are emitted.
    Events are sent as JSONL in the data field.

    Use this for real-time monitoring of trace events.
    """

    async def event_generator() -> AsyncIterator[dict]:
        """Generate SSE events from trace stream."""
        try:
            async for event in tracer.subscribe(trace_id):
                yield {
                    "event": event.event_type.value,
                    "data": tracer.event_to_jsonl(event),
                }
        except asyncio.CancelledError:
            # Client disconnected
            pass

    return EventSourceResponse(event_generator())
