"""Authentication middleware for CRA.

Provides FastAPI middleware and dependencies for authentication.
"""

import os
from contextvars import ContextVar
from typing import Any, Callable

from fastapi import Depends, HTTPException, Request, status
from fastapi.security import APIKeyHeader, HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel

from cra.auth.jwt import JWTHandler, TokenPayload, create_jwt_handler
from cra.auth.api_key import APIKeyHandler, get_api_key_handler
from cra.auth.rbac import Permission, RBACEngine, get_rbac_engine


class Principal(BaseModel):
    """Authenticated principal."""

    id: str
    type: str  # user, service, agent
    scopes: list[str] = []
    roles: list[str] = []
    org_id: str | None = None
    auth_method: str  # jwt, api_key


# Context variable for current principal
_current_principal: ContextVar[Principal | None] = ContextVar(
    "current_principal", default=None
)


def get_current_principal() -> Principal | None:
    """Get the current authenticated principal."""
    return _current_principal.get()


def set_current_principal(principal: Principal | None) -> None:
    """Set the current authenticated principal."""
    _current_principal.set(principal)


# Security schemes
bearer_scheme = HTTPBearer(auto_error=False)
api_key_header = APIKeyHeader(name="X-API-Key", auto_error=False)


class AuthMiddleware:
    """Authentication middleware for FastAPI."""

    def __init__(
        self,
        jwt_handler: JWTHandler | None = None,
        api_key_handler: APIKeyHandler | None = None,
        rbac_engine: RBACEngine | None = None,
        require_auth: bool = True,
        exempt_paths: list[str] | None = None,
    ):
        self.jwt_handler = jwt_handler or create_jwt_handler()
        self.api_key_handler = api_key_handler or get_api_key_handler()
        self.rbac_engine = rbac_engine or get_rbac_engine()
        self.require_auth = require_auth
        self.exempt_paths = set(exempt_paths or [
            "/v1/health",
            "/docs",
            "/redoc",
            "/openapi.json",
        ])

    async def __call__(self, request: Request, call_next: Callable) -> Any:
        """Process the request and authenticate."""
        # Check if path is exempt
        if request.url.path in self.exempt_paths:
            return await call_next(request)

        # Try to authenticate
        principal = await self._authenticate(request)

        if principal:
            set_current_principal(principal)
        elif self.require_auth:
            return HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Authentication required",
            )

        try:
            response = await call_next(request)
            return response
        finally:
            set_current_principal(None)

    async def _authenticate(self, request: Request) -> Principal | None:
        """Attempt to authenticate the request.

        Args:
            request: The FastAPI request

        Returns:
            Principal if authenticated, None otherwise
        """
        # Try JWT first
        auth_header = request.headers.get("Authorization")
        if auth_header and auth_header.startswith("Bearer "):
            token = auth_header[7:]
            try:
                payload = self.jwt_handler.verify_token(token)
                return Principal(
                    id=payload.sub,
                    type=payload.type,
                    scopes=payload.scopes,
                    roles=payload.roles,
                    org_id=payload.org_id,
                    auth_method="jwt",
                )
            except Exception:
                pass

        # Try API key
        api_key = request.headers.get("X-API-Key")
        if api_key:
            key = self.api_key_handler.validate_key(api_key)
            if key:
                return Principal(
                    id=key.principal_id,
                    type=key.principal_type,
                    scopes=key.scopes,
                    roles=key.roles,
                    org_id=key.org_id,
                    auth_method="api_key",
                )

        return None


# FastAPI dependencies

async def get_optional_principal(
    bearer: HTTPAuthorizationCredentials | None = Depends(bearer_scheme),
    api_key: str | None = Depends(api_key_header),
) -> Principal | None:
    """Dependency to get the current principal (optional)."""
    jwt_handler = create_jwt_handler()
    api_key_handler = get_api_key_handler()

    # Try JWT
    if bearer:
        try:
            payload = jwt_handler.verify_token(bearer.credentials)
            return Principal(
                id=payload.sub,
                type=payload.type,
                scopes=payload.scopes,
                roles=payload.roles,
                org_id=payload.org_id,
                auth_method="jwt",
            )
        except Exception:
            pass

    # Try API key
    if api_key:
        key = api_key_handler.validate_key(api_key)
        if key:
            return Principal(
                id=key.principal_id,
                type=key.principal_type,
                scopes=key.scopes,
                roles=key.roles,
                org_id=key.org_id,
                auth_method="api_key",
            )

    return None


async def get_required_principal(
    principal: Principal | None = Depends(get_optional_principal),
) -> Principal:
    """Dependency to require authentication."""
    if not principal:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Authentication required",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return principal


def require_permission(permission: str | Permission):
    """Dependency factory to require a specific permission.

    Usage:
        @app.get("/admin")
        async def admin_endpoint(
            principal: Principal = Depends(require_permission(Permission.ADMIN_USERS))
        ):
            ...
    """
    async def check_permission(
        principal: Principal = Depends(get_required_principal),
    ) -> Principal:
        rbac = get_rbac_engine()
        if not rbac.has_permission(
            principal.id,
            permission,
            principal.org_id,
            principal.roles,
        ):
            perm_str = permission.value if isinstance(permission, Permission) else permission
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail=f"Permission denied: {perm_str}",
            )
        return principal

    return check_permission


def require_any_permission(*permissions: str | Permission):
    """Dependency factory to require any of the permissions."""
    async def check_permissions(
        principal: Principal = Depends(get_required_principal),
    ) -> Principal:
        rbac = get_rbac_engine()
        if not rbac.has_any_permission(
            principal.id,
            list(permissions),
            principal.org_id,
            principal.roles,
        ):
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Permission denied",
            )
        return principal

    return check_permissions


def require_scope(scope: str):
    """Dependency factory to require a specific scope.

    Usage:
        @app.get("/tickets")
        async def tickets_endpoint(
            principal: Principal = Depends(require_scope("ticket.read"))
        ):
            ...
    """
    async def check_scope(
        principal: Principal = Depends(get_required_principal),
    ) -> Principal:
        if scope not in principal.scopes:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail=f"Scope required: {scope}",
            )
        return principal

    return check_scope
