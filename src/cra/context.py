from __future__ import annotations

import hashlib
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Dict, Tuple

from .models import ContextBlock, ISOFORMAT, now_iso


class ContextManager:
    """Manage TTL, redaction, and lineage metadata for context blocks.

    State is stored under ``lock/context_registry.json`` so subsequent resolve
    calls can enforce TTL expiry and detect content changes. When a block is
    expired it is redacted in responses and emits a TRACE artifact event via the
    runtime.
    """

    def __init__(self, lock_dir: Path | str = Path("lock")) -> None:
        self.lock_dir = Path(lock_dir)
        self.lock_dir.mkdir(parents=True, exist_ok=True)
        self.registry_file = self.lock_dir / "context_registry.json"
        self.state = self._load()

    def enforce(self, block: ContextBlock, force_refresh: bool = False) -> Tuple[ContextBlock, str]:
        """Apply TTL/redaction rules and persist registry state.

        Returns the mutated block plus a status string for telemetry:
        ``created`` (first issuance), ``active`` (previously issued and still
        valid), ``refreshed`` (content hash changed) or ``expired`` (TTL reached
        and block redacted).
        """

        now = datetime.now(timezone.utc)
        record: Dict = {} if force_refresh else self.state.get(block.block_id, {})

        content_hash = hashlib.sha256(str(block.content).encode("utf-8")).hexdigest()
        issued_at = record.get("issued_at") or now_iso()
        expires_at = record.get("expires_at")
        status = "active" if record else "created"

        # Detect content drift and refresh TTL accordingly
        if record and record.get("content_hash") != content_hash:
            status = "refreshed"
            issued_at = now_iso()
            expires_at = None

        if not expires_at:
            expires_at_dt = now + timedelta(seconds=block.ttl_seconds)
            expires_at = expires_at_dt.strftime(ISOFORMAT)

        try:
            expiry = datetime.fromisoformat(expires_at.replace("Z", "+00:00"))
        except ValueError:
            expiry = now

        if now >= expiry:
            status = "expired"
            block.redactions.append({"field": "content", "reason": "ttl_expired"})
            block.content = "[REDACTED: context ttl expired]"
            block.content_type = "text/plain"
            block.ttl_seconds = 0

        block.issued_at = issued_at
        block.expires_at = expires_at
        block.status = status

        self.state[block.block_id] = {
            "issued_at": issued_at,
            "expires_at": expires_at,
            "ttl_seconds": block.ttl_seconds,
            "content_hash": content_hash,
            "status": status,
        }
        self._save(self.state)
        return block, status

    def _load(self) -> Dict:
        if not self.registry_file.exists():
            return {}
        try:
            return json.loads(self.registry_file.read_text())
        except json.JSONDecodeError:
            return {}

    def _save(self, payload: Dict) -> None:
        self.registry_file.write_text(json.dumps(payload, indent=2))

    def refresh(self, block_id: str | None = None) -> Dict:
        """Clear registry entries to force re-issuance on next resolve.

        Returns the updated registry payload for inspection/telemetry.
        """

        if block_id:
            self.state.pop(block_id, None)
        else:
            self.state = {}
        self._save(self.state)
        return self.state

    def status(self) -> Dict:
        """Return the current registry state."""

        return self.state


__all__ = ["ContextManager"]
