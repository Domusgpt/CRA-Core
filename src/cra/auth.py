from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from .models import Session


class AuthManager:
    """Lightweight identity + scope validator.

    Identities live in ``config/identities.json`` with structure:
    {"tokens": {"token": {"principal_id": "user", "principal_type": "user", "scopes": ["scope"], "expires_at": "..."}},
     "require_token": false}
    """

    def __init__(self, config_dir: Path | str = Path("config")) -> None:
        self.config_dir = Path(config_dir)
        self.config_dir.mkdir(parents=True, exist_ok=True)
        self.identities_file = self.config_dir / "identities.json"
        self.settings = self._load()
        self.require_token = bool(self.settings.get("require_token", False))

    def validate(
        self,
        session: Session,
        token: Optional[str],
        required_scopes: Optional[List[str]] = None,
    ) -> Tuple[bool, str]:
        required_scopes = required_scopes or []

        if self.require_token and not token:
            return False, "token required"

        if not token:
            # Anonymous access is allowed unless explicitly gated
            return True, "anonymous allowed"

        identities = self.settings.get("tokens", {})
        identity = identities.get(token)
        if not identity:
            return False, "invalid token"

        expires_at = identity.get("expires_at")
        if expires_at:
            try:
                expiry = datetime.fromisoformat(expires_at.replace("Z", "+00:00"))
                if datetime.now(timezone.utc) > expiry:
                    return False, "token expired"
            except ValueError:
                return False, "invalid expiry"

        # scope check
        granted_scopes = set(identity.get("scopes", []))
        missing = [s for s in required_scopes if s not in granted_scopes]
        if missing:
            return False, f"missing scopes: {', '.join(missing)}"

        # hydrate session attributes
        session.principal_id = identity.get("principal_id", session.principal_id)
        session.principal_type = identity.get("principal_type", session.principal_type)
        session.scopes = sorted(granted_scopes or session.scopes)
        session.expires_at = identity.get("expires_at", session.expires_at)
        return True, "authorized"

    def register(
        self,
        token: str,
        principal_id: str,
        principal_type: str,
        scopes: List[str],
        expires_at: Optional[str] = None,
    ) -> Dict:
        payload = self._load()
        tokens = payload.setdefault("tokens", {})
        tokens[token] = {
            "principal_id": principal_id,
            "principal_type": principal_type,
            "scopes": scopes,
        }
        if expires_at:
            tokens[token]["expires_at"] = expires_at
        payload["require_token"] = payload.get("require_token", False)
        self._save(payload)
        self.settings = payload
        return tokens[token]

    def toggle_require_token(self, enabled: bool) -> Dict:
        payload = self._load()
        payload["require_token"] = bool(enabled)
        self._save(payload)
        self.settings = payload
        self.require_token = bool(enabled)
        return payload

    def _load(self) -> Dict:
        if not self.identities_file.exists():
            return {}
        try:
            return json.loads(self.identities_file.read_text())
        except json.JSONDecodeError:
            return {}

    def _save(self, payload: Dict) -> None:
        self.identities_file.write_text(json.dumps(payload, indent=2))


__all__ = ["AuthManager"]
