"""CARP Resolver service.

Resolves context and actions for agent tasks.
The resolver is the authoritative source for what context and actions
are permitted for a given task.
"""

from datetime import datetime
from uuid import UUID, uuid4

from cra.core.carp import (
    ActionKind,
    AllowedAction,
    AtlasRef,
    CARPRequest,
    CARPResponse,
    ContextBlock,
    ContentType,
    DenyRule,
    MergeRules,
    NextStep,
    Resolution,
    ResolveResponsePayload,
    Session,
    TraceContext,
)
from cra.core.trace import EventType
from cra.runtime.services.session_manager import SessionManager
from cra.runtime.services.tracer import Tracer
from cra.version import CARP_VERSION


class Resolver:
    """CARP resolution service.

    Handles the resolution of context and actions for agent tasks.
    All resolutions emit TRACE events for auditability.
    """

    def __init__(self, tracer: Tracer, session_manager: SessionManager) -> None:
        """Initialize the resolver.

        Args:
            tracer: The tracer service for event emission
            session_manager: The session manager for session tracking
        """
        self._tracer = tracer
        self._session_manager = session_manager

    async def resolve(self, request: CARPRequest) -> CARPResponse:
        """Resolve a CARP request.

        This is the main entry point for CARP resolution.

        Args:
            request: The CARP request envelope

        Returns:
            The CARP response envelope with resolution

        Raises:
            SessionExpiredError: If the session has expired
            SessionNotFoundError: If the session doesn't exist
        """
        # Validate session
        session = await self._session_manager.get_session(request.session.session_id)

        # Create span for this resolution
        span_id = uuid4()

        # Emit resolution requested event
        await self._tracer.emit(
            event_type=EventType.CARP_RESOLVE_REQUESTED,
            trace_id=request.trace.trace_id,
            session_id=request.session.session_id,
            span_id=span_id,
            parent_span_id=request.trace.parent_span_id,
            atlas=AtlasRef(id=request.atlas.id, version=request.atlas.version)
            if request.atlas
            else None,
            payload={
                "goal": request.payload.task.goal,
                "risk_tier": request.payload.task.risk_tier.value,
                "target_platforms": request.payload.task.target_platforms,
            },
        )

        # Perform resolution (currently returns a demo resolution)
        resolution = await self._perform_resolution(request, session)

        # Increment resolution count
        await self._session_manager.increment_resolution_count(request.session.session_id)

        # Create response envelope
        response = CARPResponse(
            id=uuid4(),
            time=datetime.utcnow(),
            session=request.session,
            atlas=request.atlas,
            payload=ResolveResponsePayload(resolution=resolution),
            trace=TraceContext(
                trace_id=request.trace.trace_id,
                span_id=span_id,
                parent_span_id=request.trace.parent_span_id,
            ),
        )

        # Emit resolution returned event
        await self._tracer.emit(
            event_type=EventType.CARP_RESOLVE_RETURNED,
            trace_id=request.trace.trace_id,
            session_id=request.session.session_id,
            span_id=span_id,
            parent_span_id=request.trace.parent_span_id,
            atlas=AtlasRef(id=request.atlas.id, version=request.atlas.version)
            if request.atlas
            else None,
            payload={
                "resolution_id": str(resolution.resolution_id),
                "confidence": resolution.confidence,
                "context_block_count": len(resolution.context_blocks),
                "allowed_action_count": len(resolution.allowed_actions),
                "deny_rule_count": len(resolution.denylist),
            },
        )

        return response

    async def _perform_resolution(
        self, request: CARPRequest, session: Session
    ) -> Resolution:
        """Perform the actual resolution logic.

        For Phase 0, this returns a demo resolution.
        In Phase 1+, this will:
        - Load relevant Atlas
        - Evaluate policies
        - Apply scopes and permissions
        - Generate context blocks
        - Determine allowed actions

        Args:
            request: The CARP request
            session: The validated session

        Returns:
            The resolution result
        """
        task = request.payload.task
        resolution_id = uuid4()

        # Demo context block based on the goal
        context_blocks = [
            ContextBlock(
                block_id="cra-guidelines",
                purpose="CRA agent behavior guidelines",
                ttl_seconds=3600,
                content_type=ContentType.MARKDOWN,
                content=self._get_agent_guidelines(),
            ),
            ContextBlock(
                block_id="task-context",
                purpose=f"Context for: {task.goal}",
                ttl_seconds=1800,
                content_type=ContentType.MARKDOWN,
                content=f"""## Task Context

**Goal:** {task.goal}
**Risk Tier:** {task.risk_tier.value}
**Constraints:** {', '.join(task.constraints) if task.constraints else 'None specified'}

### Guidelines
- Follow the principle of least privilege
- All actions must be approved before execution
- TRACE output is authoritative
""",
            ),
        ]

        # Demo allowed actions
        allowed_actions = [
            AllowedAction(
                action_id="cra.echo",
                kind=ActionKind.TOOL_CALL,
                adapter="builtin",
                description="Echo a message for testing",
                json_schema={
                    "type": "object",
                    "properties": {"message": {"type": "string"}},
                    "required": ["message"],
                },
                requires_approval=False,
            ),
        ]

        # High-risk tasks require approval
        if task.risk_tier.value == "high":
            for action in allowed_actions:
                action.requires_approval = True

        # Demo deny rules
        denylist = [
            DenyRule(
                pattern="*.production.*",
                reason="Production access requires explicit scopes",
            ),
            DenyRule(
                pattern="rm -rf *",
                reason="Destructive operations not permitted",
            ),
        ]

        # Compute confidence (demo: based on task complexity)
        confidence = 0.85 if task.risk_tier.value == "low" else 0.70

        return Resolution(
            resolution_id=resolution_id,
            confidence=confidence,
            context_blocks=context_blocks,
            allowed_actions=allowed_actions,
            denylist=denylist,
            merge_rules=MergeRules(),
            next_steps=[
                NextStep(
                    step="Review the allowed actions",
                    expected_artifacts=["action_plan.md"],
                ),
                NextStep(
                    step="Execute approved actions",
                    expected_artifacts=["execution_log.json"],
                ),
            ],
        )

    def _get_agent_guidelines(self) -> str:
        """Get the standard agent guidelines.

        Returns:
            Markdown content with agent guidelines
        """
        return """## CRA Agent Contract

### Core Rules
1. **Always resolve via CARP** before taking any action
2. **Never guess** tool usage or API behavior
3. **TRACE is authoritative** - LLM narration is not
4. **Respect TTLs** on context blocks
5. **Honor the denylist** - never attempt denied patterns

### Execution Protocol
1. Request resolution for your task
2. Review allowed actions and constraints
3. Execute only approved actions
4. Monitor TRACE output for confirmation

### Error Handling
- If an action fails, check TRACE for details
- Do not retry without re-resolution
- Report errors through the proper channels
"""


# Global resolver instance
_resolver: Resolver | None = None


def get_resolver(tracer: Tracer, session_manager: SessionManager) -> Resolver:
    """Get the global resolver instance."""
    global _resolver
    if _resolver is None:
        _resolver = Resolver(tracer, session_manager)
    return _resolver
