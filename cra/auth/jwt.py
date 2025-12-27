"""JWT authentication for CRA.

Handles token generation, validation, and refresh.
"""

import os
from datetime import datetime, timedelta, timezone
from typing import Any

from pydantic import BaseModel, Field
import jwt


class JWTConfig(BaseModel):
    """JWT configuration."""

    secret_key: str = Field(
        default_factory=lambda: os.getenv("CRA_JWT_SECRET", "dev-secret-change-in-production")
    )
    algorithm: str = "HS256"
    access_token_expire_minutes: int = 60
    refresh_token_expire_days: int = 7
    issuer: str = "cra-runtime"
    audience: str = "cra-agents"


class TokenPayload(BaseModel):
    """JWT token payload."""

    sub: str  # Subject (principal ID)
    type: str  # Principal type (user, service, agent)
    scopes: list[str] = Field(default_factory=list)
    roles: list[str] = Field(default_factory=list)
    org_id: str | None = None
    exp: datetime
    iat: datetime
    iss: str
    aud: str
    jti: str | None = None  # Token ID for revocation


class JWTHandler:
    """Handles JWT token operations."""

    def __init__(self, config: JWTConfig | None = None):
        self.config = config or JWTConfig()
        self._revoked_tokens: set[str] = set()

    def create_access_token(
        self,
        principal_id: str,
        principal_type: str,
        scopes: list[str] | None = None,
        roles: list[str] | None = None,
        org_id: str | None = None,
        expires_delta: timedelta | None = None,
    ) -> str:
        """Create an access token.

        Args:
            principal_id: The principal identifier
            principal_type: Type of principal (user, service, agent)
            scopes: List of granted scopes
            roles: List of roles
            org_id: Organization ID
            expires_delta: Custom expiration time

        Returns:
            Encoded JWT token
        """
        import uuid

        now = datetime.now(timezone.utc)
        if expires_delta:
            expire = now + expires_delta
        else:
            expire = now + timedelta(minutes=self.config.access_token_expire_minutes)

        payload = {
            "sub": principal_id,
            "type": principal_type,
            "scopes": scopes or [],
            "roles": roles or [],
            "org_id": org_id,
            "exp": expire,
            "iat": now,
            "iss": self.config.issuer,
            "aud": self.config.audience,
            "jti": str(uuid.uuid4()),
        }

        return jwt.encode(
            payload,
            self.config.secret_key,
            algorithm=self.config.algorithm,
        )

    def create_refresh_token(
        self,
        principal_id: str,
        principal_type: str,
    ) -> str:
        """Create a refresh token.

        Args:
            principal_id: The principal identifier
            principal_type: Type of principal

        Returns:
            Encoded refresh token
        """
        import uuid

        now = datetime.now(timezone.utc)
        expire = now + timedelta(days=self.config.refresh_token_expire_days)

        payload = {
            "sub": principal_id,
            "type": principal_type,
            "token_type": "refresh",
            "exp": expire,
            "iat": now,
            "iss": self.config.issuer,
            "aud": self.config.audience,
            "jti": str(uuid.uuid4()),
        }

        return jwt.encode(
            payload,
            self.config.secret_key,
            algorithm=self.config.algorithm,
        )

    def verify_token(self, token: str) -> TokenPayload:
        """Verify and decode a token.

        Args:
            token: The JWT token to verify

        Returns:
            Decoded token payload

        Raises:
            jwt.InvalidTokenError: If token is invalid
            ValueError: If token is revoked
        """
        payload = jwt.decode(
            token,
            self.config.secret_key,
            algorithms=[self.config.algorithm],
            audience=self.config.audience,
            issuer=self.config.issuer,
        )

        # Check if token is revoked
        jti = payload.get("jti")
        if jti and jti in self._revoked_tokens:
            raise ValueError("Token has been revoked")

        return TokenPayload(
            sub=payload["sub"],
            type=payload["type"],
            scopes=payload.get("scopes", []),
            roles=payload.get("roles", []),
            org_id=payload.get("org_id"),
            exp=datetime.fromtimestamp(payload["exp"], tz=timezone.utc),
            iat=datetime.fromtimestamp(payload["iat"], tz=timezone.utc),
            iss=payload["iss"],
            aud=payload["aud"],
            jti=payload.get("jti"),
        )

    def revoke_token(self, token: str) -> None:
        """Revoke a token.

        Args:
            token: The token to revoke
        """
        try:
            payload = jwt.decode(
                token,
                self.config.secret_key,
                algorithms=[self.config.algorithm],
                options={"verify_exp": False},
            )
            if jti := payload.get("jti"):
                self._revoked_tokens.add(jti)
        except jwt.InvalidTokenError:
            pass

    def refresh_access_token(self, refresh_token: str) -> str:
        """Refresh an access token using a refresh token.

        Args:
            refresh_token: The refresh token

        Returns:
            New access token

        Raises:
            jwt.InvalidTokenError: If refresh token is invalid
            ValueError: If not a refresh token
        """
        payload = jwt.decode(
            refresh_token,
            self.config.secret_key,
            algorithms=[self.config.algorithm],
            audience=self.config.audience,
            issuer=self.config.issuer,
        )

        if payload.get("token_type") != "refresh":
            raise ValueError("Not a refresh token")

        return self.create_access_token(
            principal_id=payload["sub"],
            principal_type=payload["type"],
        )


def create_jwt_handler(config: JWTConfig | None = None) -> JWTHandler:
    """Create a JWT handler instance."""
    return JWTHandler(config)
