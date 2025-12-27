"""Unit tests for CARP types."""

from datetime import datetime
from uuid import uuid4

import pytest

from cra.core.carp import (
    ActionKind,
    AllowedAction,
    CARPRequest,
    ContextBlock,
    ContentType,
    Principal,
    PrincipalType,
    ResolveRequestPayload,
    Resolution,
    RiskTier,
    Session,
    Task,
    TraceContext,
)


class TestPrincipal:
    """Tests for Principal model."""

    def test_valid_principal(self):
        """Test creating a valid principal."""
        principal = Principal(type=PrincipalType.USER, id="user-123")
        assert principal.type == PrincipalType.USER
        assert principal.id == "user-123"

    def test_principal_requires_id(self):
        """Test that principal requires non-empty id."""
        with pytest.raises(ValueError):
            Principal(type=PrincipalType.USER, id="")


class TestTask:
    """Tests for Task model."""

    def test_task_with_goal(self):
        """Test creating a task with just a goal."""
        task = Task(goal="Deploy to staging")
        assert task.goal == "Deploy to staging"
        assert task.risk_tier == RiskTier.MEDIUM

    def test_task_with_all_fields(self):
        """Test creating a task with all fields."""
        task = Task(
            goal="Deploy to production",
            inputs=[],
            constraints=["must pass tests"],
            target_platforms=["openai.tools"],
            risk_tier=RiskTier.HIGH,
        )
        assert task.risk_tier == RiskTier.HIGH
        assert len(task.constraints) == 1


class TestContextBlock:
    """Tests for ContextBlock model."""

    def test_context_block(self):
        """Test creating a context block."""
        block = ContextBlock(
            block_id="test-block",
            purpose="Test purpose",
            content="Test content",
        )
        assert block.block_id == "test-block"
        assert block.ttl_seconds == 3600  # Default
        assert block.content_type == ContentType.MARKDOWN  # Default


class TestAllowedAction:
    """Tests for AllowedAction model."""

    def test_allowed_action(self):
        """Test creating an allowed action."""
        action = AllowedAction(
            action_id="test.action",
            kind=ActionKind.TOOL_CALL,
            adapter="builtin",
            json_schema={"type": "object"},
        )
        assert action.action_id == "test.action"
        assert action.requires_approval is False


class TestResolution:
    """Tests for Resolution model."""

    def test_resolution_confidence_bounds(self):
        """Test that confidence must be between 0 and 1."""
        resolution = Resolution(
            resolution_id=uuid4(),
            confidence=0.85,
        )
        assert resolution.confidence == 0.85

        with pytest.raises(ValueError):
            Resolution(resolution_id=uuid4(), confidence=1.5)

        with pytest.raises(ValueError):
            Resolution(resolution_id=uuid4(), confidence=-0.1)


class TestCARPRequest:
    """Tests for CARPRequest envelope."""

    def test_carp_request_envelope(self):
        """Test creating a CARP request envelope."""
        session = Session(
            session_id=uuid4(),
            principal=Principal(type=PrincipalType.USER, id="test-user"),
            scopes=["carp.resolve"],
        )
        trace = TraceContext(
            trace_id=uuid4(),
            span_id=uuid4(),
        )
        payload = ResolveRequestPayload(
            task=Task(goal="Test goal"),
        )

        request = CARPRequest(
            id=uuid4(),
            time=datetime.utcnow(),
            session=session,
            payload=payload,
            trace=trace,
        )

        assert request.type == "carp.request"
        assert request.carp_version == "1.0"
