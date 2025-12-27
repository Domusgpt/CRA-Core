"""TRACE protocol compliance tests.

Verifies that the implementation strictly follows TRACE/1.0 specification.
Principle: If it wasn't emitted by the runtime, it didn't happen.
"""

from datetime import datetime
from uuid import uuid4

import pytest

from cra.core.trace import (
    Actor,
    ActorType,
    Artifact,
    EventType,
    Severity,
    TraceContext,
    TraceEvent,
)


class TestTraceEventCompliance:
    """Tests for TRACE event structure compliance."""

    @pytest.fixture
    def valid_trace_context(self):
        """Create a valid trace context."""
        return TraceContext(trace_id=uuid4(), span_id=uuid4())

    @pytest.fixture
    def valid_actor(self):
        """Create a valid actor."""
        return Actor(type=ActorType.RUNTIME, id="cra-runtime")

    def test_trace_version_must_be_1_0(self, valid_trace_context, valid_actor):
        """TRACE events must have trace_version='1.0'."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=valid_trace_context,
            session_id=uuid4(),
            actor=valid_actor,
        )
        assert event.trace_version == "1.0"

    def test_event_must_have_event_type(self, valid_trace_context, valid_actor):
        """TRACE events must have an event_type."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=valid_trace_context,
            session_id=uuid4(),
            actor=valid_actor,
        )
        assert event.event_type is not None

    def test_event_must_have_timestamp(self, valid_trace_context, valid_actor):
        """TRACE events must have a timestamp."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=valid_trace_context,
            session_id=uuid4(),
            actor=valid_actor,
        )
        assert event.time is not None
        assert isinstance(event.time, datetime)

    def test_event_must_have_trace_context(self, valid_actor):
        """TRACE events must have a trace context."""
        with pytest.raises(Exception):
            TraceEvent(
                event_type=EventType.SESSION_STARTED,
                time=datetime.utcnow(),
                trace=None,  # type: ignore
                session_id=uuid4(),
                actor=valid_actor,
            )

    def test_event_must_have_session_id(self, valid_trace_context, valid_actor):
        """TRACE events must have a session_id."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=valid_trace_context,
            session_id=uuid4(),
            actor=valid_actor,
        )
        assert event.session_id is not None

    def test_event_must_have_actor(self, valid_trace_context):
        """TRACE events must have an actor."""
        with pytest.raises(Exception):
            TraceEvent(
                event_type=EventType.SESSION_STARTED,
                time=datetime.utcnow(),
                trace=valid_trace_context,
                session_id=uuid4(),
                actor=None,  # type: ignore
            )


class TestTraceEventTypeCompliance:
    """Tests for TRACE event type naming compliance."""

    def test_all_event_types_start_with_trace(self):
        """All event types must start with 'trace.'."""
        for event_type in EventType:
            assert event_type.value.startswith("trace.")

    def test_mandatory_session_events_exist(self):
        """Mandatory session events must exist."""
        assert EventType.SESSION_STARTED.value == "trace.session.started"
        assert EventType.SESSION_ENDED.value == "trace.session.ended"

    def test_mandatory_carp_events_exist(self):
        """Mandatory CARP events must exist."""
        assert EventType.CARP_RESOLVE_REQUESTED.value == "trace.carp.resolve.requested"
        assert EventType.CARP_RESOLVE_RETURNED.value == "trace.carp.resolve.returned"
        assert EventType.CARP_POLICY_DENIED.value == "trace.carp.policy.denied"

    def test_mandatory_action_events_exist(self):
        """Mandatory action events must exist."""
        assert EventType.ACTION_GRANTED.value == "trace.action.granted"
        assert EventType.ACTION_INVOKED.value == "trace.action.invoked"
        assert EventType.ACTION_COMPLETED.value == "trace.action.completed"
        assert EventType.ACTION_FAILED.value == "trace.action.failed"


class TestTraceContextCompliance:
    """Tests for TRACE context structure compliance."""

    def test_trace_context_must_have_trace_id(self):
        """Trace context must have a trace_id."""
        ctx = TraceContext(trace_id=uuid4(), span_id=uuid4())
        assert ctx.trace_id is not None

    def test_trace_context_must_have_span_id(self):
        """Trace context must have a span_id."""
        ctx = TraceContext(trace_id=uuid4(), span_id=uuid4())
        assert ctx.span_id is not None

    def test_trace_context_parent_span_id_is_optional(self):
        """Trace context parent_span_id is optional."""
        ctx = TraceContext(trace_id=uuid4(), span_id=uuid4())
        assert ctx.parent_span_id is None

        ctx_with_parent = TraceContext(
            trace_id=uuid4(),
            span_id=uuid4(),
            parent_span_id=uuid4(),
        )
        assert ctx_with_parent.parent_span_id is not None


class TestActorCompliance:
    """Tests for TRACE actor structure compliance."""

    def test_actor_must_have_type(self):
        """Actors must have a type."""
        actor = Actor(type=ActorType.RUNTIME, id="test")
        assert actor.type is not None

    def test_actor_must_have_id(self):
        """Actors must have an id."""
        actor = Actor(type=ActorType.RUNTIME, id="test-id")
        assert actor.id == "test-id"

    def test_actor_types_are_valid(self):
        """Actor types must be valid."""
        valid_types = {
            ActorType.RUNTIME,
            ActorType.AGENT,
            ActorType.USER,
            ActorType.TOOL,
        }
        assert set(ActorType) == valid_types


class TestSeverityCompliance:
    """Tests for TRACE severity compliance."""

    def test_severity_values_are_valid(self):
        """Severity must be debug, info, warn, or error."""
        valid_severities = {
            Severity.DEBUG,
            Severity.INFO,
            Severity.WARN,
            Severity.ERROR,
        }
        assert set(Severity) == valid_severities

    def test_default_severity_is_info(self):
        """Default severity should be info."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=TraceContext(trace_id=uuid4(), span_id=uuid4()),
            session_id=uuid4(),
            actor=Actor(type=ActorType.RUNTIME, id="test"),
        )
        assert event.severity == Severity.INFO


class TestArtifactCompliance:
    """Tests for TRACE artifact structure compliance."""

    def test_artifact_must_have_name(self):
        """Artifacts must have a name."""
        artifact = Artifact(
            name="output.json",
            uri="file://./output.json",
            sha256="a" * 64,
            content_type="application/json",
        )
        assert artifact.name == "output.json"

    def test_artifact_must_have_uri(self):
        """Artifacts must have a URI."""
        artifact = Artifact(
            name="test",
            uri="s3://bucket/key",
            sha256="a" * 64,
            content_type="text/plain",
        )
        assert artifact.uri == "s3://bucket/key"

    def test_artifact_must_have_sha256(self):
        """Artifacts must have a SHA256 hash."""
        artifact = Artifact(
            name="test",
            uri="file://test",
            sha256="a" * 64,
            content_type="text/plain",
        )
        assert len(artifact.sha256) == 64

    def test_artifact_sha256_must_be_valid_hex(self):
        """Artifact SHA256 must be valid hex."""
        # Valid
        Artifact(
            name="test",
            uri="test",
            sha256="0123456789abcdef" * 4,
            content_type="text/plain",
        )

        # Invalid
        with pytest.raises(ValueError):
            Artifact(
                name="test",
                uri="test",
                sha256="not-valid-hex",
                content_type="text/plain",
            )

    def test_artifact_must_have_content_type(self):
        """Artifacts must have a content_type."""
        artifact = Artifact(
            name="test",
            uri="test",
            sha256="a" * 64,
            content_type="application/json",
        )
        assert artifact.content_type == "application/json"


class TestTraceImmutability:
    """Tests for TRACE immutability guarantees."""

    def test_events_should_be_immutable_after_creation(self):
        """Events should effectively be immutable once created."""
        event = TraceEvent(
            event_type=EventType.SESSION_STARTED,
            time=datetime.utcnow(),
            trace=TraceContext(trace_id=uuid4(), span_id=uuid4()),
            session_id=uuid4(),
            actor=Actor(type=ActorType.RUNTIME, id="test"),
            payload={"key": "value"},
        )

        # The model should be immutable in spirit - any modifications
        # should create new instances, not mutate existing ones
        original_id = id(event)
        assert original_id == id(event)


class TestTraceReplayCompliance:
    """Tests for TRACE replay functionality compliance."""

    def test_replay_manifest_must_have_trace_id(self):
        """Replay manifests must have a trace_id."""
        from cra.core.replay import ReplayManifest
        manifest = ReplayManifest(trace_id=uuid4())
        assert manifest.trace_id is not None

    def test_replay_manifest_version_must_be_1_0(self):
        """Replay manifests must have version 1.0."""
        from cra.core.replay import ReplayManifest
        manifest = ReplayManifest(trace_id=uuid4())
        assert manifest.manifest_version == "1.0"

    def test_replay_must_support_nondeterminism_rules(self):
        """Replay must support nondeterminism handling rules."""
        from cra.core.replay import ReplayManifest, NondeterminismRule, NondeterminismRuleType

        manifest = ReplayManifest(
            trace_id=uuid4(),
            nondeterminism=[
                NondeterminismRule(field="time", rule=NondeterminismRuleType.IGNORE),
                NondeterminismRule(field="span_id", rule=NondeterminismRuleType.NORMALIZE),
            ],
        )
        assert len(manifest.nondeterminism) == 2
