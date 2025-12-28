"""Authentication and authorization for CRA.

Provides JWT and API key authentication, plus RBAC.
"""

from cra.auth.jwt import JWTHandler, JWTConfig, TokenPayload
from cra.auth.api_key import APIKeyHandler, APIKey
from cra.auth.rbac import RBACEngine, Role, Permission
from cra.auth.middleware import AuthMiddleware, get_current_principal

__all__ = [
    "JWTHandler",
    "JWTConfig",
    "TokenPayload",
    "APIKeyHandler",
    "APIKey",
    "RBACEngine",
    "Role",
    "Permission",
    "AuthMiddleware",
    "get_current_principal",
]
