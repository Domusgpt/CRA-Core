"""Atlas management endpoints."""

from pathlib import Path
from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Query, status
from pydantic import BaseModel, Field

from cra.runtime.services.atlas_service import AtlasService, get_atlas_service

router = APIRouter(prefix="/v1/atlases", tags=["atlases"])


class AtlasSummary(BaseModel):
    """Summary of an Atlas."""

    id: str
    version: str
    name: str
    description: str = ""
    capabilities: list[str] = Field(default_factory=list)
    context_packs: int = 0
    policies: int = 0
    adapters: list[str] = Field(default_factory=list)
    certification: dict[str, bool] = Field(default_factory=dict)


class AtlasListResponse(BaseModel):
    """Response for listing Atlases."""

    atlases: list[AtlasSummary]
    count: int


class LoadAtlasRequest(BaseModel):
    """Request to load an Atlas."""

    path: str


class LoadAtlasResponse(BaseModel):
    """Response after loading an Atlas."""

    success: bool
    atlas: AtlasSummary | None = None
    error: str | None = None


class EmitPlatformRequest(BaseModel):
    """Request to emit for a platform."""

    resolution_id: str
    platform: str


def get_atlas_service_dep() -> AtlasService:
    """Dependency for getting the Atlas service."""
    return get_atlas_service()


@router.get("", response_model=AtlasListResponse)
async def list_atlases(
    service: AtlasService = Depends(get_atlas_service_dep),
) -> AtlasListResponse:
    """List all registered Atlases.

    Returns summary information for each Atlas.
    """
    atlases = service.list_atlases()
    summaries = [
        AtlasSummary(**service.get_atlas_summary(atlas))
        for atlas in atlases
    ]
    return AtlasListResponse(atlases=summaries, count=len(summaries))


@router.post("/load", response_model=LoadAtlasResponse)
async def load_atlas(
    request: LoadAtlasRequest,
    service: AtlasService = Depends(get_atlas_service_dep),
) -> LoadAtlasResponse:
    """Load an Atlas from a path.

    The path should point to a directory containing atlas.json.
    """
    try:
        path = Path(request.path)
        if not path.exists():
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Path not found: {request.path}",
            )

        atlas = service.load_atlas(path)
        summary = AtlasSummary(**service.get_atlas_summary(atlas))
        return LoadAtlasResponse(success=True, atlas=summary)

    except Exception as e:
        return LoadAtlasResponse(success=False, error=str(e))


@router.get("/{atlas_id}", response_model=AtlasSummary)
async def get_atlas(
    atlas_id: str,
    service: AtlasService = Depends(get_atlas_service_dep),
) -> AtlasSummary:
    """Get an Atlas by ID.

    Returns detailed information about the Atlas.
    """
    atlas = service.get_atlas(atlas_id)
    if not atlas:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Atlas not found: {atlas_id}",
        )

    return AtlasSummary(**service.get_atlas_summary(atlas))


@router.delete("/{atlas_id}")
async def unload_atlas(
    atlas_id: str,
    service: AtlasService = Depends(get_atlas_service_dep),
) -> dict[str, Any]:
    """Unload an Atlas.

    Removes the Atlas from the registry.
    """
    if not service.unload_atlas(atlas_id):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Atlas not found: {atlas_id}",
        )

    return {"success": True, "atlas_id": atlas_id}


@router.get("/{atlas_id}/context")
async def get_atlas_context(
    atlas_id: str,
    capability: str | None = Query(None, description="Filter by capability"),
    service: AtlasService = Depends(get_atlas_service_dep),
) -> dict[str, Any]:
    """Get context blocks from an Atlas.

    Returns the context packs as context blocks.
    """
    atlas = service.get_atlas(atlas_id)
    if not atlas:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Atlas not found: {atlas_id}",
        )

    blocks = service.get_context_blocks(atlas, capability)
    return {
        "atlas_id": atlas_id,
        "blocks": [block.model_dump() for block in blocks],
        "count": len(blocks),
    }


@router.get("/{atlas_id}/actions")
async def get_atlas_actions(
    atlas_id: str,
    capability: str | None = Query(None, description="Filter by capability"),
    service: AtlasService = Depends(get_atlas_service_dep),
) -> dict[str, Any]:
    """Get allowed actions from an Atlas.

    Returns actions defined in the Atlas adapters.
    """
    atlas = service.get_atlas(atlas_id)
    if not atlas:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Atlas not found: {atlas_id}",
        )

    actions = service.get_allowed_actions(atlas, capability)
    return {
        "atlas_id": atlas_id,
        "actions": [action.model_dump() for action in actions],
        "count": len(actions),
    }


@router.get("/{atlas_id}/emit/{platform}")
async def emit_for_platform(
    atlas_id: str,
    platform: str,
    service: AtlasService = Depends(get_atlas_service_dep),
) -> dict[str, Any]:
    """Emit Atlas in platform-specific format.

    Generates output for the specified platform adapter.
    Supported platforms: openai, anthropic, google_adk, mcp
    """
    atlas = service.get_atlas(atlas_id)
    if not atlas:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Atlas not found: {atlas_id}",
        )

    # Create a dummy resolution for the Atlas
    from uuid import uuid4
    from cra.core.carp import Resolution, MergeRules

    resolution = Resolution(
        resolution_id=uuid4(),
        confidence=0.95,
        context_blocks=service.get_context_blocks(atlas),
        allowed_actions=service.get_allowed_actions(atlas),
        denylist=service.get_deny_rules(atlas),
        merge_rules=MergeRules(),
        next_steps=[],
    )

    try:
        output = service.emit_for_platform(resolution, platform, atlas)
        return {
            "atlas_id": atlas_id,
            "platform": platform,
            "output": output,
        }
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e),
        )
