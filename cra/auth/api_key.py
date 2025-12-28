"""API key authentication for CRA.

Provides simple API key validation for service-to-service auth.
"""

import hashlib
import os
import secrets
from datetime import datetime, timezone
from typing import Any

from pydantic import BaseModel, Field


class APIKey(BaseModel):
    """API key model."""

    key_id: str
    key_hash: str  # SHA-256 hash of the key
    name: str
    principal_id: str
    principal_type: str = "service"
    scopes: list[str] = Field(default_factory=list)
    roles: list[str] = Field(default_factory=list)
    org_id: str | None = None
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    expires_at: datetime | None = None
    last_used_at: datetime | None = None
    is_active: bool = True


class APIKeyHandler:
    """Handles API key operations."""

    def __init__(self):
        self._keys: dict[str, APIKey] = {}
        self._key_hash_index: dict[str, str] = {}  # hash -> key_id

    def generate_key(
        self,
        name: str,
        principal_id: str,
        principal_type: str = "service",
        scopes: list[str] | None = None,
        roles: list[str] | None = None,
        org_id: str | None = None,
        expires_at: datetime | None = None,
    ) -> tuple[str, APIKey]:
        """Generate a new API key.

        Args:
            name: Human-readable name for the key
            principal_id: The principal this key represents
            principal_type: Type of principal
            scopes: List of granted scopes
            roles: List of roles
            org_id: Organization ID
            expires_at: Optional expiration time

        Returns:
            Tuple of (raw_key, APIKey model)
        """
        # Generate a secure random key
        raw_key = f"cra_{secrets.token_urlsafe(32)}"
        key_hash = self._hash_key(raw_key)
        key_id = f"key_{secrets.token_urlsafe(8)}"

        api_key = APIKey(
            key_id=key_id,
            key_hash=key_hash,
            name=name,
            principal_id=principal_id,
            principal_type=principal_type,
            scopes=scopes or [],
            roles=roles or [],
            org_id=org_id,
            expires_at=expires_at,
        )

        self._keys[key_id] = api_key
        self._key_hash_index[key_hash] = key_id

        return raw_key, api_key

    def validate_key(self, raw_key: str) -> APIKey | None:
        """Validate an API key.

        Args:
            raw_key: The raw API key to validate

        Returns:
            APIKey if valid, None otherwise
        """
        key_hash = self._hash_key(raw_key)
        key_id = self._key_hash_index.get(key_hash)

        if not key_id:
            return None

        api_key = self._keys.get(key_id)
        if not api_key:
            return None

        # Check if active
        if not api_key.is_active:
            return None

        # Check expiration
        if api_key.expires_at:
            if datetime.now(timezone.utc) > api_key.expires_at:
                return None

        # Update last used
        api_key.last_used_at = datetime.now(timezone.utc)

        return api_key

    def revoke_key(self, key_id: str) -> bool:
        """Revoke an API key.

        Args:
            key_id: The key ID to revoke

        Returns:
            True if revoked, False if not found
        """
        if key_id in self._keys:
            self._keys[key_id].is_active = False
            return True
        return False

    def get_key(self, key_id: str) -> APIKey | None:
        """Get an API key by ID.

        Args:
            key_id: The key ID

        Returns:
            APIKey if found, None otherwise
        """
        return self._keys.get(key_id)

    def list_keys(self, principal_id: str | None = None) -> list[APIKey]:
        """List API keys.

        Args:
            principal_id: Optional filter by principal

        Returns:
            List of API keys
        """
        keys = list(self._keys.values())
        if principal_id:
            keys = [k for k in keys if k.principal_id == principal_id]
        return keys

    def _hash_key(self, raw_key: str) -> str:
        """Hash an API key."""
        return hashlib.sha256(raw_key.encode()).hexdigest()


# Singleton instance
_api_key_handler: APIKeyHandler | None = None


def get_api_key_handler() -> APIKeyHandler:
    """Get the API key handler singleton."""
    global _api_key_handler
    if _api_key_handler is None:
        _api_key_handler = APIKeyHandler()
    return _api_key_handler
