"""CARP protocol compliance tests.

Verifies that the implementation strictly follows CARP/1.0 specification.
"""

from datetime import datetime
from uuid import uuid4

import pytest

from cra.core.carp import (
    CARPRequest,
    CARPResponse,
    Principal,
    PrincipalType,
    ResolveRequestPayload,
    RiskTier,
    Session,
    Task,
    TraceContext,
)
from cra.core.validation import SchemaValidator, SchemaValidationError


class TestCARPEnvelopeCompliance:
    """Tests for CARP envelope structure compliance."""

    @pytest.fixture
    def validator(self):
        """Get a schema validator."""
        return SchemaValidator()

    @pytest.fixture
    def valid_session(self):
        """Create a valid session."""
        return Session(
            session_id=uuid4(),
            principal=Principal(type=PrincipalType.USER, id="test-user"),
            scopes=["carp.resolve"],
        )

    @pytest.fixture
    def valid_trace(self):
        """Create a valid trace context."""
        return TraceContext(trace_id=uuid4(), span_id=uuid4())

    def test_carp_version_must_be_1_0(self, valid_session, valid_trace):
        """CARP requests must have carp_version='1.0'."""
        request = CARPRequest(
            id=uuid4(),
            time=datetime.utcnow(),
            session=valid_session,
            payload=ResolveRequestPayload(task=Task(goal="Test")),
            trace=valid_trace,
        )
        assert request.carp_version == "1.0"

    def test_request_type_must_be_carp_request(self, valid_session, valid_trace):
        """CARP requests must have type='carp.request'."""
        request = CARPRequest(
            id=uuid4(),
            time=datetime.utcnow(),
            session=valid_session,
            payload=ResolveRequestPayload(task=Task(goal="Test")),
            trace=valid_trace,
        )
        assert request.type == "carp.request"

    def test_response_type_must_be_carp_response(self):
        """CARP responses must have type='carp.response'."""
        # This is enforced by the Literal type
        from cra.core.carp import CARPResponse
        assert CARPResponse.model_fields["type"].default == "carp.response"

    def test_envelope_must_have_uuid_id(self, valid_session, valid_trace):
        """CARP envelopes must have a valid UUID id."""
        request = CARPRequest(
            id=uuid4(),
            time=datetime.utcnow(),
            session=valid_session,
            payload=ResolveRequestPayload(task=Task(goal="Test")),
            trace=valid_trace,
        )
        assert request.id is not None

    def test_envelope_must_have_timestamp(self, valid_session, valid_trace):
        """CARP envelopes must have a timestamp."""
        request = CARPRequest(
            id=uuid4(),
            time=datetime.utcnow(),
            session=valid_session,
            payload=ResolveRequestPayload(task=Task(goal="Test")),
            trace=valid_trace,
        )
        assert request.time is not None
        assert isinstance(request.time, datetime)

    def test_envelope_must_have_session(self, valid_trace):
        """CARP envelopes must have a session block."""
        with pytest.raises(Exception):  # Pydantic validation error
            CARPRequest(
                id=uuid4(),
                time=datetime.utcnow(),
                session=None,  # type: ignore
                payload=ResolveRequestPayload(task=Task(goal="Test")),
                trace=valid_trace,
            )

    def test_envelope_must_have_trace_context(self, valid_session):
        """CARP envelopes must have a trace context."""
        with pytest.raises(Exception):  # Pydantic validation error
            CARPRequest(
                id=uuid4(),
                time=datetime.utcnow(),
                session=valid_session,
                payload=ResolveRequestPayload(task=Task(goal="Test")),
                trace=None,  # type: ignore
            )


class TestCARPSessionCompliance:
    """Tests for CARP session structure compliance."""

    def test_session_must_have_session_id(self):
        """Sessions must have a session_id."""
        session = Session(
            session_id=uuid4(),
            principal=Principal(type=PrincipalType.USER, id="test"),
            scopes=[],
        )
        assert session.session_id is not None

    def test_session_must_have_principal(self):
        """Sessions must have a principal."""
        session = Session(
            session_id=uuid4(),
            principal=Principal(type=PrincipalType.USER, id="test"),
            scopes=[],
        )
        assert session.principal is not None

    def test_principal_types_are_valid(self):
        """Principal types must be user, service, or agent."""
        valid_types = {PrincipalType.USER, PrincipalType.SERVICE, PrincipalType.AGENT}
        assert set(PrincipalType) == valid_types

    def test_scopes_must_be_list(self):
        """Scopes must be a list of strings."""
        session = Session(
            session_id=uuid4(),
            principal=Principal(type=PrincipalType.USER, id="test"),
            scopes=["read", "write"],
        )
        assert isinstance(session.scopes, list)


class TestCARPTaskCompliance:
    """Tests for CARP task structure compliance."""

    def test_task_must_have_goal(self):
        """Tasks must have a goal."""
        task = Task(goal="Deploy to staging")
        assert task.goal == "Deploy to staging"

    def test_task_goal_must_be_non_empty(self):
        """Task goal must be non-empty."""
        with pytest.raises(ValueError):
            Task(goal="")

    def test_risk_tier_values_are_valid(self):
        """Risk tier must be low, medium, or high."""
        valid_tiers = {RiskTier.LOW, RiskTier.MEDIUM, RiskTier.HIGH}
        assert set(RiskTier) == valid_tiers

    def test_default_risk_tier_is_medium(self):
        """Default risk tier should be medium."""
        task = Task(goal="Test")
        assert task.risk_tier == RiskTier.MEDIUM


class TestCARPResolutionCompliance:
    """Tests for CARP resolution structure compliance."""

    def test_resolution_must_have_resolution_id(self):
        """Resolutions must have a resolution_id."""
        from cra.core.carp import Resolution
        resolution = Resolution(resolution_id=uuid4(), confidence=0.9)
        assert resolution.resolution_id is not None

    def test_resolution_confidence_must_be_valid(self):
        """Confidence must be between 0 and 1."""
        from cra.core.carp import Resolution

        # Valid values
        Resolution(resolution_id=uuid4(), confidence=0.0)
        Resolution(resolution_id=uuid4(), confidence=0.5)
        Resolution(resolution_id=uuid4(), confidence=1.0)

        # Invalid values
        with pytest.raises(ValueError):
            Resolution(resolution_id=uuid4(), confidence=-0.1)
        with pytest.raises(ValueError):
            Resolution(resolution_id=uuid4(), confidence=1.1)

    def test_resolution_must_have_context_blocks_list(self):
        """Resolutions must have a context_blocks list."""
        from cra.core.carp import Resolution
        resolution = Resolution(resolution_id=uuid4(), confidence=0.9)
        assert isinstance(resolution.context_blocks, list)

    def test_resolution_must_have_allowed_actions_list(self):
        """Resolutions must have an allowed_actions list."""
        from cra.core.carp import Resolution
        resolution = Resolution(resolution_id=uuid4(), confidence=0.9)
        assert isinstance(resolution.allowed_actions, list)

    def test_resolution_must_have_denylist(self):
        """Resolutions must have a denylist."""
        from cra.core.carp import Resolution
        resolution = Resolution(resolution_id=uuid4(), confidence=0.9)
        assert isinstance(resolution.denylist, list)


class TestCARPContextBlockCompliance:
    """Tests for CARP context block compliance."""

    def test_context_block_must_have_block_id(self):
        """Context blocks must have a block_id."""
        from cra.core.carp import ContextBlock
        block = ContextBlock(block_id="test", purpose="Test", content="Content")
        assert block.block_id == "test"

    def test_context_block_must_have_purpose(self):
        """Context blocks must have a purpose."""
        from cra.core.carp import ContextBlock
        block = ContextBlock(block_id="test", purpose="Test purpose", content="Content")
        assert block.purpose == "Test purpose"

    def test_context_block_must_have_ttl(self):
        """Context blocks must have a TTL."""
        from cra.core.carp import ContextBlock
        block = ContextBlock(block_id="test", purpose="Test", content="Content")
        assert block.ttl_seconds >= 0

    def test_context_block_default_ttl_is_3600(self):
        """Default TTL should be 3600 seconds (1 hour)."""
        from cra.core.carp import ContextBlock
        block = ContextBlock(block_id="test", purpose="Test", content="Content")
        assert block.ttl_seconds == 3600


class TestCARPAllowedActionCompliance:
    """Tests for CARP allowed action compliance."""

    def test_action_must_have_action_id(self):
        """Allowed actions must have an action_id."""
        from cra.core.carp import AllowedAction, ActionKind
        action = AllowedAction(
            action_id="test.action",
            kind=ActionKind.TOOL_CALL,
            adapter="builtin",
        )
        assert action.action_id == "test.action"

    def test_action_must_have_kind(self):
        """Allowed actions must have a kind."""
        from cra.core.carp import AllowedAction, ActionKind
        action = AllowedAction(
            action_id="test.action",
            kind=ActionKind.TOOL_CALL,
            adapter="builtin",
        )
        assert action.kind == ActionKind.TOOL_CALL

    def test_action_kinds_are_valid(self):
        """Action kinds must be valid."""
        from cra.core.carp import ActionKind
        valid_kinds = {
            ActionKind.TOOL_CALL,
            ActionKind.MCP_CALL,
            ActionKind.CLI_COMMAND,
            ActionKind.AGENT_TOOL,
        }
        assert set(ActionKind) == valid_kinds

    def test_action_requires_approval_default_is_false(self):
        """Default requires_approval should be false."""
        from cra.core.carp import AllowedAction, ActionKind
        action = AllowedAction(
            action_id="test.action",
            kind=ActionKind.TOOL_CALL,
            adapter="builtin",
        )
        assert action.requires_approval is False


class TestCARPDenyRuleCompliance:
    """Tests for CARP deny rule compliance."""

    def test_deny_rule_must_have_pattern(self):
        """Deny rules must have a pattern."""
        from cra.core.carp import DenyRule
        rule = DenyRule(pattern="*.production.*", reason="Not allowed")
        assert rule.pattern == "*.production.*"

    def test_deny_rule_must_have_reason(self):
        """Deny rules must have a reason."""
        from cra.core.carp import DenyRule
        rule = DenyRule(pattern="*", reason="Denied for testing")
        assert rule.reason == "Denied for testing"
