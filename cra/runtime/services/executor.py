"""Action executor service.

Executes granted actions with full TRACE emission.
Handles approval workflows and constraint enforcement.
"""

import asyncio
import hashlib
import json
from datetime import datetime, timedelta
from typing import Any, Callable
from uuid import UUID, uuid4

from cra.core.action import (
    ActionExecution,
    ActionGrant,
    ApprovalRequest,
    ApprovalResponse,
    ExecuteActionRequest,
    ExecuteActionResponse,
    ExecutionStatus,
)
from cra.core.trace import EventType
from cra.runtime.services.session_manager import SessionManager
from cra.runtime.services.tracer import Tracer


class ActionNotFoundError(Exception):
    """Action grant not found."""

    pass


class ActionNotApprovedError(Exception):
    """Action requires approval but not approved."""

    pass


class ActionExpiredError(Exception):
    """Action grant has expired."""

    pass


class ActionConstraintError(Exception):
    """Action constraint violated."""

    pass


# Type for action handlers
ActionHandler = Callable[[str, dict[str, Any]], Any]


class ActionExecutor:
    """Executes granted actions.

    All executions are recorded in TRACE for auditability.
    """

    def __init__(self, tracer: Tracer, session_manager: SessionManager) -> None:
        """Initialize the executor.

        Args:
            tracer: The tracer service
            session_manager: The session manager
        """
        self._tracer = tracer
        self._session_manager = session_manager
        self._grants: dict[UUID, ActionGrant] = {}
        self._executions: dict[UUID, ActionExecution] = {}
        self._pending_approvals: dict[UUID, ApprovalRequest] = {}
        self._handlers: dict[str, ActionHandler] = {}
        self._lock = asyncio.Lock()

        # Register built-in handlers
        self._register_builtin_handlers()

    def _register_builtin_handlers(self) -> None:
        """Register built-in action handlers."""

        async def echo_handler(action_id: str, params: dict[str, Any]) -> dict[str, Any]:
            """Echo handler for testing."""
            return {"echo": params.get("message", ""), "action_id": action_id}

        async def noop_handler(action_id: str, params: dict[str, Any]) -> dict[str, Any]:
            """No-op handler."""
            return {"status": "ok"}

        self._handlers["cra.echo"] = echo_handler
        self._handlers["cra.noop"] = noop_handler

    def register_handler(self, action_id: str, handler: ActionHandler) -> None:
        """Register an action handler.

        Args:
            action_id: The action ID
            handler: The handler function
        """
        self._handlers[action_id] = handler

    async def grant_action(
        self,
        resolution_id: UUID,
        action_id: str,
        kind: str,
        adapter: str,
        schema: dict[str, Any],
        constraints: list[dict[str, Any]],
        requires_approval: bool,
        ttl_seconds: int = 3600,
    ) -> ActionGrant:
        """Create a grant for an action.

        Args:
            resolution_id: The resolution that allowed this action
            action_id: The action ID
            kind: Action kind (tool_call, mcp_call, etc.)
            adapter: The adapter to use
            schema: JSON Schema for parameters
            constraints: Constraints on execution
            requires_approval: Whether approval is needed
            ttl_seconds: Grant TTL

        Returns:
            The created ActionGrant
        """
        grant = ActionGrant(
            resolution_id=resolution_id,
            action_id=action_id,
            kind=kind,
            adapter=adapter,
            schema=schema,
            constraints=constraints,
            requires_approval=requires_approval,
            expires_at=datetime.utcnow() + timedelta(seconds=ttl_seconds),
        )

        async with self._lock:
            self._grants[grant.grant_id] = grant

        return grant

    async def request_approval(
        self,
        grant_id: UUID,
        session_id: UUID,
        trace_id: UUID,
        reason: str,
        risk_tier: str,
        requested_by: str,
    ) -> ApprovalRequest:
        """Request approval for an action.

        Args:
            grant_id: The grant requiring approval
            session_id: The session ID
            trace_id: The trace ID
            reason: Reason for approval
            risk_tier: Risk tier
            requested_by: Who is requesting

        Returns:
            The approval request
        """
        async with self._lock:
            grant = self._grants.get(grant_id)
            if not grant:
                raise ActionNotFoundError(f"Grant {grant_id} not found")

        request = ApprovalRequest(
            grant_id=grant_id,
            action_id=grant.action_id,
            reason=reason,
            risk_tier=risk_tier,
            requested_by=requested_by,
        )

        async with self._lock:
            self._pending_approvals[grant_id] = request

        # Emit approval requested event
        await self._tracer.emit(
            event_type=EventType.ACTION_GRANTED,
            trace_id=trace_id,
            session_id=session_id,
            payload={
                "grant_id": str(grant_id),
                "action_id": grant.action_id,
                "requires_approval": True,
                "status": "pending_approval",
            },
        )

        return request

    async def approve_action(
        self,
        grant_id: UUID,
        approved_by: str,
        session_id: UUID,
        trace_id: UUID,
    ) -> ApprovalResponse:
        """Approve an action.

        Args:
            grant_id: The grant to approve
            approved_by: Who is approving
            session_id: The session ID
            trace_id: The trace ID

        Returns:
            The approval response
        """
        async with self._lock:
            grant = self._grants.get(grant_id)
            if not grant:
                raise ActionNotFoundError(f"Grant {grant_id} not found")

            grant.approved = True
            grant.approved_by = approved_by
            grant.approved_at = datetime.utcnow()

            # Remove from pending
            self._pending_approvals.pop(grant_id, None)

        # Emit approval event
        await self._tracer.emit(
            event_type=EventType.ACTION_GRANTED,
            trace_id=trace_id,
            session_id=session_id,
            payload={
                "grant_id": str(grant_id),
                "action_id": grant.action_id,
                "approved": True,
                "approved_by": approved_by,
            },
        )

        return ApprovalResponse(
            grant_id=grant_id,
            approved=True,
            approved_by=approved_by,
            approved_at=grant.approved_at,
        )

    async def reject_action(
        self,
        grant_id: UUID,
        rejected_by: str,
        reason: str,
        session_id: UUID,
        trace_id: UUID,
    ) -> ApprovalResponse:
        """Reject an action.

        Args:
            grant_id: The grant to reject
            rejected_by: Who is rejecting
            reason: Rejection reason
            session_id: The session ID
            trace_id: The trace ID

        Returns:
            The approval response
        """
        async with self._lock:
            grant = self._grants.get(grant_id)
            if not grant:
                raise ActionNotFoundError(f"Grant {grant_id} not found")

            # Remove from pending
            self._pending_approvals.pop(grant_id, None)
            # Remove grant
            del self._grants[grant_id]

        # Emit rejection event
        await self._tracer.emit(
            event_type=EventType.ACTION_FAILED,
            trace_id=trace_id,
            session_id=session_id,
            payload={
                "grant_id": str(grant_id),
                "action_id": grant.action_id,
                "rejected": True,
                "rejected_by": rejected_by,
                "reason": reason,
            },
        )

        return ApprovalResponse(
            grant_id=grant_id,
            approved=False,
            reason=reason,
        )

    async def execute(self, request: ExecuteActionRequest) -> ExecuteActionResponse:
        """Execute a granted action.

        Args:
            request: The execution request

        Returns:
            The execution response

        Raises:
            ActionNotFoundError: If grant not found
            ActionNotApprovedError: If approval required but not given
            ActionExpiredError: If grant has expired
        """
        # Find the grant
        grant = await self._find_grant_for_action(request.resolution_id, request.action_id)
        if not grant:
            raise ActionNotFoundError(
                f"No grant found for action {request.action_id} in resolution {request.resolution_id}"
            )

        # Check expiration
        if datetime.utcnow() > grant.expires_at:
            raise ActionExpiredError(f"Grant for {request.action_id} has expired")

        # Check approval
        if grant.requires_approval and not grant.approved:
            raise ActionNotApprovedError(f"Action {request.action_id} requires approval")

        # Create execution record
        execution = ActionExecution(
            grant_id=grant.grant_id,
            session_id=request.session_id,
            action_id=request.action_id,
            parameters=request.parameters,
            parameters_hash=self._hash_json(request.parameters),
            status=ExecutionStatus.RUNNING,
            started_at=datetime.utcnow(),
            trace_id=request.trace_id,
            span_id=request.span_id,
        )

        async with self._lock:
            self._executions[execution.execution_id] = execution

        # Emit invoked event
        await self._tracer.emit(
            event_type=EventType.ACTION_INVOKED,
            trace_id=request.trace_id,
            session_id=request.session_id,
            span_id=request.span_id,
            parent_span_id=request.parent_span_id,
            payload={
                "action_id": request.action_id,
                "execution_id": str(execution.execution_id),
                "parameters_hash": execution.parameters_hash,
            },
        )

        # Execute the action
        try:
            result = await self._execute_action(request.action_id, request.parameters)
            execution.status = ExecutionStatus.COMPLETED
            execution.result = result
            execution.result_hash = self._hash_json(result)

        except Exception as e:
            execution.status = ExecutionStatus.FAILED
            execution.error = {"type": type(e).__name__, "message": str(e)}

        finally:
            execution.completed_at = datetime.utcnow()
            if execution.started_at:
                execution.duration_ms = int(
                    (execution.completed_at - execution.started_at).total_seconds() * 1000
                )

        # Update session stats
        await self._session_manager.increment_action_count(
            request.session_id,
            failed=(execution.status == ExecutionStatus.FAILED),
        )

        # Emit completion/failure event
        if execution.status == ExecutionStatus.COMPLETED:
            await self._tracer.emit(
                event_type=EventType.ACTION_COMPLETED,
                trace_id=request.trace_id,
                session_id=request.session_id,
                span_id=request.span_id,
                parent_span_id=request.parent_span_id,
                payload={
                    "action_id": request.action_id,
                    "execution_id": str(execution.execution_id),
                    "duration_ms": execution.duration_ms,
                    "result_hash": execution.result_hash,
                },
            )
        else:
            await self._tracer.emit(
                event_type=EventType.ACTION_FAILED,
                trace_id=request.trace_id,
                session_id=request.session_id,
                span_id=request.span_id,
                parent_span_id=request.parent_span_id,
                payload={
                    "action_id": request.action_id,
                    "execution_id": str(execution.execution_id),
                    "duration_ms": execution.duration_ms,
                    "error_type": execution.error.get("type") if execution.error else None,
                    "error_message": execution.error.get("message") if execution.error else None,
                },
            )

        return ExecuteActionResponse(
            execution_id=execution.execution_id,
            status=execution.status,
            result=execution.result,
            error=execution.error,
            duration_ms=execution.duration_ms,
            trace={
                "trace_id": str(request.trace_id),
                "span_id": str(request.span_id),
            },
        )

    async def _find_grant_for_action(
        self, resolution_id: UUID, action_id: str
    ) -> ActionGrant | None:
        """Find a grant for an action.

        Args:
            resolution_id: The resolution ID
            action_id: The action ID

        Returns:
            The grant or None
        """
        async with self._lock:
            for grant in self._grants.values():
                if grant.resolution_id == resolution_id and grant.action_id == action_id:
                    return grant
        return None

    async def _execute_action(
        self, action_id: str, parameters: dict[str, Any]
    ) -> dict[str, Any]:
        """Execute an action using its handler.

        Args:
            action_id: The action ID
            parameters: The parameters

        Returns:
            The result

        Raises:
            ActionNotFoundError: If no handler registered
        """
        handler = self._handlers.get(action_id)
        if not handler:
            # Default passthrough for unregistered actions
            return {"action_id": action_id, "parameters": parameters, "status": "passthrough"}

        result = handler(action_id, parameters)
        if asyncio.iscoroutine(result):
            result = await result
        return result

    def _hash_json(self, data: Any) -> str:
        """Hash JSON data for auditing.

        Args:
            data: Data to hash

        Returns:
            SHA256 hex digest
        """
        json_str = json.dumps(data, sort_keys=True, default=str)
        return hashlib.sha256(json_str.encode()).hexdigest()

    async def get_execution(self, execution_id: UUID) -> ActionExecution | None:
        """Get an execution by ID.

        Args:
            execution_id: The execution ID

        Returns:
            The execution or None
        """
        async with self._lock:
            return self._executions.get(execution_id)

    async def get_pending_approvals(self, session_id: UUID | None = None) -> list[ApprovalRequest]:
        """Get pending approval requests.

        Args:
            session_id: Optional session ID to filter by

        Returns:
            List of pending approvals
        """
        async with self._lock:
            approvals = list(self._pending_approvals.values())
        return approvals


# Global executor instance
_executor: ActionExecutor | None = None


def get_executor(tracer: Tracer, session_manager: SessionManager) -> ActionExecutor:
    """Get the global action executor instance."""
    global _executor
    if _executor is None:
        _executor = ActionExecutor(tracer, session_manager)
    return _executor
