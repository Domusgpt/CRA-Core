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
from cra.core.policy import PolicyContext, PolicyEffect, PolicyEngine
from cra.core.trace import EventType, Severity
from cra.runtime.services.session_manager import SessionManager
from cra.runtime.services.tracer import Tracer
from cra.version import CARP_VERSION


class PolicyDeniedError(Exception):
    """Raised when a policy denies a resolution."""

    def __init__(self, reason: str, rule_id: str | None = None) -> None:
        super().__init__(reason)
        self.reason = reason
        self.rule_id = rule_id


class Resolver:
    """CARP resolution service.

    Handles the resolution of context and actions for agent tasks.
    All resolutions emit TRACE events for auditability.
    Integrates with PolicyEngine for governance.
    """

    def __init__(
        self,
        tracer: Tracer,
        session_manager: SessionManager,
        policy_engine: PolicyEngine | None = None,
    ) -> None:
        """Initialize the resolver.

        Args:
            tracer: The tracer service for event emission
            session_manager: The session manager for session tracking
            policy_engine: Optional policy engine for governance
        """
        self._tracer = tracer
        self._session_manager = session_manager
        self._policy_engine = policy_engine

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
            PolicyDeniedError: If policy denies the resolution
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

        # Evaluate policies if engine is available
        policy_decision = None
        if self._policy_engine:
            policy_context = PolicyContext(
                session_id=request.session.session_id,
                principal_type=request.session.principal.type.value,
                principal_id=request.session.principal.id,
                scopes=request.session.scopes,
                risk_tier=request.payload.task.risk_tier.value,
                goal=request.payload.task.goal,
            )
            policy_decision = self._policy_engine.evaluate(policy_context)

            # If denied, emit event and raise
            if policy_decision.effect == PolicyEffect.DENY:
                await self._tracer.emit(
                    event_type=EventType.CARP_POLICY_DENIED,
                    trace_id=request.trace.trace_id,
                    session_id=request.session.session_id,
                    span_id=span_id,
                    severity=Severity.WARN,
                    payload={
                        "rule_id": policy_decision.rule_id,
                        "reason": policy_decision.reason,
                        "violations": [v.model_dump() for v in policy_decision.violations],
                    },
                )
                raise PolicyDeniedError(
                    policy_decision.reason, policy_decision.rule_id
                )

        # Perform resolution
        resolution = await self._perform_resolution(request, session, policy_decision)

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
                "requires_approval": any(a.requires_approval for a in resolution.allowed_actions),
                "policy_effect": policy_decision.effect.value if policy_decision else "allow",
            },
        )

        return response

    async def _perform_resolution(
        self,
        request: CARPRequest,
        session: Session,
        policy_decision=None,
    ) -> Resolution:
        """Perform the actual resolution logic.

        Integrates with PolicyEngine for:
        - Approval requirements
        - Redactions
        - Constraints

        Args:
            request: The CARP request
            session: The validated session
            policy_decision: Optional policy decision

        Returns:
            The resolution result
        """
        task = request.payload.task
        resolution_id = uuid4()

        # Build context blocks
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

        # Add policy context block if there's a policy decision
        if policy_decision:
            policy_content = f"""## Policy Evaluation

**Effect:** {policy_decision.effect.value}
**Requires Approval:** {policy_decision.requires_approval}
"""
            if policy_decision.redactions:
                policy_content += f"\n**Redacted Fields:** {', '.join(policy_decision.redactions)}"
            if policy_decision.constraints:
                policy_content += f"\n**Constraints:** {policy_decision.constraints}"

            context_blocks.append(
                ContextBlock(
                    block_id="policy-context",
                    purpose="Policy evaluation results",
                    ttl_seconds=1800,
                    content_type=ContentType.MARKDOWN,
                    content=policy_content,
                )
            )

        # Build allowed actions
        requires_approval = (
            policy_decision.requires_approval if policy_decision else False
        ) or task.risk_tier.value == "high"

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
                requires_approval=requires_approval,
            ),
            AllowedAction(
                action_id="cra.noop",
                kind=ActionKind.TOOL_CALL,
                adapter="builtin",
                description="No-operation action for testing",
                json_schema={"type": "object", "properties": {}},
                requires_approval=False,
            ),
        ]

        # Build deny rules from policy
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

        # Add deny rules from policy violations
        if policy_decision and policy_decision.violations:
            for violation in policy_decision.violations:
                if violation.details.get("pattern"):
                    denylist.append(
                        DenyRule(
                            pattern=violation.details["pattern"],
                            reason=violation.reason,
                        )
                    )

        # Compute confidence
        confidence = 0.85
        if task.risk_tier.value == "medium":
            confidence = 0.75
        elif task.risk_tier.value == "high":
            confidence = 0.65

        # Reduce confidence if policy added constraints
        if policy_decision and policy_decision.effect == PolicyEffect.ALLOW_WITH_CONSTRAINTS:
            confidence *= 0.9

        return Resolution(
            resolution_id=resolution_id,
            confidence=confidence,
            context_blocks=context_blocks,
            allowed_actions=allowed_actions,
            denylist=denylist,
            merge_rules=MergeRules(),
            next_steps=[
                NextStep(
                    step="Review the allowed actions and constraints",
                    expected_artifacts=["action_plan.md"],
                ),
                NextStep(
                    step="Request approval if required",
                    expected_artifacts=["approval_request.json"],
                ) if requires_approval else NextStep(
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
3. Request approval if required
4. Execute only approved actions
5. Monitor TRACE output for confirmation

### Error Handling
- If an action fails, check TRACE for details
- Do not retry without re-resolution
- Report errors through the proper channels
"""


# Global resolver instance
_resolver: Resolver | None = None


def get_resolver(
    tracer: Tracer,
    session_manager: SessionManager,
    policy_engine: PolicyEngine | None = None,
) -> Resolver:
    """Get the global resolver instance."""
    global _resolver
    if _resolver is None:
        _resolver = Resolver(tracer, session_manager, policy_engine)
    return _resolver
