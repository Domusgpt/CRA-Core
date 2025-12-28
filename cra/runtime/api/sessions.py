"""Session management endpoints."""

from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, status

from cra.core.session import (
    CreateSessionRequest,
    CreateSessionResponse,
    EndSessionResponse,
)
from cra.runtime.services.session_manager import (
    SessionExpiredError,
    SessionManager,
    SessionNotFoundError,
)
from cra.runtime.api.dependencies import get_session_manager_dep

router = APIRouter(prefix="/v1/sessions", tags=["sessions"])


@router.post("", response_model=CreateSessionResponse, status_code=status.HTTP_201_CREATED)
async def create_session(
    request: CreateSessionRequest,
    session_manager: SessionManager = Depends(get_session_manager_dep),
) -> CreateSessionResponse:
    """Create a new session.

    Sessions track the lifecycle of an agent's interaction with the CRA runtime.
    Each session gets a unique trace_id for correlating all events.

    The session will automatically expire after the TTL.
    """
    return await session_manager.create_session(request)


@router.post("/{session_id}/end", response_model=EndSessionResponse)
async def end_session(
    session_id: UUID,
    session_manager: SessionManager = Depends(get_session_manager_dep),
) -> EndSessionResponse:
    """End a session.

    Marks the session as ended and returns summary statistics.
    This emits a trace.session.ended event.
    """
    try:
        return await session_manager.end_session(session_id)
    except SessionNotFoundError as e:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=str(e))
    except SessionExpiredError as e:
        raise HTTPException(status_code=status.HTTP_410_GONE, detail=str(e))
