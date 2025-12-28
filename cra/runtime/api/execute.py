"""Action execution endpoints."""

from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel

from cra.core.action import (
    ApprovalResponse,
    ExecuteActionRequest,
    ExecuteActionResponse,
    ExecutionStatus,
)
from cra.runtime.services.executor import (
    ActionExecutor,
    ActionExpiredError,
    ActionNotApprovedError,
    ActionNotFoundError,
)
from cra.runtime.api.dependencies import get_executor_dep

router = APIRouter(prefix="/v1/carp", tags=["execute"])


@router.post("/execute", response_model=ExecuteActionResponse)
async def execute_action(
    request: ExecuteActionRequest,
    executor: ActionExecutor = Depends(get_executor_dep),
) -> ExecuteActionResponse:
    """Execute a granted action.

    Executes an action that was previously granted in a CARP resolution.
    The action must:
    - Have a valid grant from a resolution
    - Not be expired
    - Be approved (if approval was required)

    TRACE events emitted:
    - trace.action.invoked (when execution starts)
    - trace.action.completed (on success)
    - trace.action.failed (on failure)
    """
    try:
        return await executor.execute(request)
    except ActionNotFoundError as e:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=str(e))
    except ActionNotApprovedError as e:
        raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail=str(e))
    except ActionExpiredError as e:
        raise HTTPException(status_code=status.HTTP_410_GONE, detail=str(e))


class ApproveActionRequest(BaseModel):
    """Request to approve an action."""

    grant_id: UUID
    session_id: UUID
    trace_id: UUID
    approved_by: str


class RejectActionRequest(BaseModel):
    """Request to reject an action."""

    grant_id: UUID
    session_id: UUID
    trace_id: UUID
    rejected_by: str
    reason: str


@router.post("/actions/{grant_id}/approve", response_model=ApprovalResponse)
async def approve_action(
    grant_id: UUID,
    request: ApproveActionRequest,
    executor: ActionExecutor = Depends(get_executor_dep),
) -> ApprovalResponse:
    """Approve a pending action.

    Approves an action that requires approval before execution.
    Only actions with requires_approval=true need this step.
    """
    try:
        return await executor.approve_action(
            grant_id=grant_id,
            approved_by=request.approved_by,
            session_id=request.session_id,
            trace_id=request.trace_id,
        )
    except ActionNotFoundError as e:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=str(e))


@router.post("/actions/{grant_id}/reject", response_model=ApprovalResponse)
async def reject_action(
    grant_id: UUID,
    request: RejectActionRequest,
    executor: ActionExecutor = Depends(get_executor_dep),
) -> ApprovalResponse:
    """Reject a pending action.

    Rejects an action, preventing its execution.
    The grant will be removed.
    """
    try:
        return await executor.reject_action(
            grant_id=grant_id,
            rejected_by=request.rejected_by,
            reason=request.reason,
            session_id=request.session_id,
            trace_id=request.trace_id,
        )
    except ActionNotFoundError as e:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=str(e))


class PendingApproval(BaseModel):
    """A pending approval request."""

    grant_id: UUID
    action_id: str
    reason: str
    risk_tier: str
    requested_by: str
    requested_at: str


class PendingApprovalsResponse(BaseModel):
    """Response with pending approvals."""

    approvals: list[PendingApproval]
    count: int


@router.get("/actions/pending", response_model=PendingApprovalsResponse)
async def list_pending_approvals(
    session_id: UUID | None = None,
    executor: ActionExecutor = Depends(get_executor_dep),
) -> PendingApprovalsResponse:
    """List pending action approvals.

    Returns all actions waiting for approval.
    """
    pending = await executor.get_pending_approvals(session_id)
    return PendingApprovalsResponse(
        approvals=[
            PendingApproval(
                grant_id=p.grant_id,
                action_id=p.action_id,
                reason=p.reason,
                risk_tier=p.risk_tier,
                requested_by=p.requested_by,
                requested_at=p.requested_at.isoformat(),
            )
            for p in pending
        ],
        count=len(pending),
    )


class ExecutionStatusResponse(BaseModel):
    """Response with execution status."""

    execution_id: UUID
    status: ExecutionStatus
    action_id: str
    started_at: str | None
    completed_at: str | None
    duration_ms: int | None


@router.get("/executions/{execution_id}", response_model=ExecutionStatusResponse)
async def get_execution_status(
    execution_id: UUID,
    executor: ActionExecutor = Depends(get_executor_dep),
) -> ExecutionStatusResponse:
    """Get the status of an action execution.

    Returns the current status and timing information.
    """
    execution = await executor.get_execution(execution_id)
    if not execution:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Execution {execution_id} not found",
        )

    return ExecutionStatusResponse(
        execution_id=execution.execution_id,
        status=execution.status,
        action_id=execution.action_id,
        started_at=execution.started_at.isoformat() if execution.started_at else None,
        completed_at=execution.completed_at.isoformat() if execution.completed_at else None,
        duration_ms=execution.duration_ms,
    )
