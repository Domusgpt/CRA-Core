"""Unit tests for TRACE types."""

from datetime import datetime
from uuid import uuid4

import pytest

from cra.core.trace import (
    Actor,
    ActorType,
    Artifact,
    EventType,
    ReplayManifest,
    Severity,
    TraceContext,
    TraceEvent,
)


class TestTraceContext:
    """Tests for TraceContext model."""

    def test_trace_context(self):
        """Test creating a trace context."""
        ctx = TraceContext(
            trace_id=uuid4(),
            span_id=uuid4(),
        )
        assert ctx.parent_span_id is None

    def test_trace_context_with_parent(self):
        """Test creating a trace context with parent."""
        parent_span = uuid4()
        ctx = TraceContext(
            trace_id=uuid4(),
            span_id=uuid4(),
            parent_span_id=parent_span,
        )
        assert ctx.parent_span_id == parent_span


class TestActor:
    """Tests for Actor model."""

    def test_actor(self):
        """Test creating an actor."""
        actor = Actor(type=ActorType.RUNTIME, id="cra-runtime")
        assert actor.type == ActorType.RUNTIME
        assert actor.id == "cra-runtime"


class TestArtifact:
    """Tests for Artifact model."""

    def test_artifact(self):
        """Test creating an artifact."""
        artifact = Artifact(
            name="output.json",
            uri="file://./output.json",
            sha256="a" * 64,
            content_type="application/json",
        )
        assert artifact.name == "output.json"

    def test_artifact_invalid_sha256(self):
        """Test that artifact requires valid SHA256."""
        with pytest.raises(ValueError):
            Artifact(
                name="test",
                uri="test",
                sha256="invalid",
                content_type="text/plain",
            )


class TestTraceEvent:
    """Tests for TraceEvent model."""

    def test_trace_event(self):
        """Test creating a trace event."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=TraceContext(trace_id=uuid4(), span_id=uuid4()),
            session_id=uuid4(),
            actor=Actor(type=ActorType.RUNTIME, id="cra-runtime"),
        )
        assert event.trace_version == "1.0"
        assert event.severity == Severity.INFO

    def test_trace_event_with_payload(self):
        """Test creating a trace event with payload."""
        event = TraceEvent(
            event_type=EventType.CARP_RESOLVE_RETURNED,
            time=datetime.utcnow(),
            trace=TraceContext(trace_id=uuid4(), span_id=uuid4()),
            session_id=uuid4(),
            actor=Actor(type=ActorType.RUNTIME, id="cra-runtime"),
            payload={"resolution_id": "abc-123", "confidence": 0.95},
        )
        assert event.payload["confidence"] == 0.95


class TestEventType:
    """Tests for EventType enum."""

    def test_event_types_have_correct_prefix(self):
        """Test that all event types start with 'trace.'"""
        for event_type in EventType:
            assert event_type.value.startswith("trace.")


class TestReplayManifest:
    """Tests for ReplayManifest model."""

    def test_replay_manifest(self):
        """Test creating a replay manifest."""
        manifest = ReplayManifest(
            trace_id=uuid4(),
            created_at=datetime.utcnow(),
            description="Test trace",
        )
        assert manifest.manifest_version == "1.0"
        assert manifest.artifacts == []
        assert manifest.nondeterminism == []
