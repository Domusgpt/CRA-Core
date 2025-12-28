"""Health check endpoint."""

import time
from datetime import datetime

from fastapi import APIRouter
from pydantic import BaseModel

from cra.version import CARP_VERSION, TRACE_VERSION, __version__

router = APIRouter(prefix="/v1", tags=["health"])

# Track server start time
_start_time = time.time()


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    carp_version: str
    trace_version: str
    uptime_seconds: float
    timestamp: datetime


@router.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    """Check runtime health.

    Returns basic health information about the CRA runtime.
    """
    return HealthResponse(
        status="healthy",
        version=__version__,
        carp_version=CARP_VERSION,
        trace_version=TRACE_VERSION,
        uptime_seconds=time.time() - _start_time,
        timestamp=datetime.utcnow(),
    )
