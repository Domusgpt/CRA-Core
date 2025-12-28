"""CARP resolution endpoints."""

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import ValidationError

from cra.core.carp import CARPRequest, CARPResponse
from cra.runtime.services.resolver import Resolver
from cra.runtime.services.session_manager import SessionExpiredError, SessionNotFoundError
from cra.runtime.api.dependencies import get_resolver_dep

router = APIRouter(prefix="/v1/carp", tags=["carp"])


@router.post("/resolve", response_model=CARPResponse)
async def resolve(
    request: CARPRequest,
    resolver: Resolver = Depends(get_resolver_dep),
) -> CARPResponse:
    """Resolve context and actions for a task.

    This is the main CARP resolution endpoint. It:
    1. Validates the session
    2. Resolves appropriate context blocks
    3. Determines allowed actions
    4. Applies deny rules
    5. Emits TRACE events

    The response contains a Resolution Bundle with:
    - context_blocks: Minimal, TTL-bounded context
    - allowed_actions: Permitted actions with schemas
    - denylist: Patterns to never attempt
    - next_steps: Suggested workflow
    """
    try:
        return await resolver.resolve(request)
    except SessionNotFoundError as e:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=str(e))
    except SessionExpiredError as e:
        raise HTTPException(status_code=status.HTTP_410_GONE, detail=str(e))
    except ValidationError as e:
        raise HTTPException(
            status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
            detail={"message": "Invalid CARP request", "errors": e.errors()},
        )
