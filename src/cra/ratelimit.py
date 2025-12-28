from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Dict, Tuple


class RateLimiter:
    """Lightweight per-action rate limiter persisted to lock/ratelimits.json."""

    def __init__(self, lock_dir: Path | str = Path("lock")) -> None:
        self.lock_dir = Path(lock_dir)
        self.lock_dir.mkdir(parents=True, exist_ok=True)
        self.state_file = self.lock_dir / "ratelimits.json"
        self.state = self._load()

    def check(self, action_id: str, limit: int, window_seconds: int = 3600) -> Tuple[bool, str]:
        now = time.time()
        record = self.state.get(action_id, {"count": 0, "window_start": now})

        if now - record.get("window_start", now) > window_seconds:
            record = {"count": 0, "window_start": now}

        if record["count"] >= limit:
            return False, f"rate limit exceeded ({limit}/{window_seconds}s)"

        record["count"] += 1
        record["window_start"] = record.get("window_start", now)
        self.state[action_id] = record
        self._save()
        return True, f"count {record['count']} of {limit}"

    def _load(self) -> Dict:
        if not self.state_file.exists():
            return {}
        try:
            return json.loads(self.state_file.read_text())
        except json.JSONDecodeError:
            return {}

    def _save(self) -> None:
        self.state_file.write_text(json.dumps(self.state, indent=2))


__all__ = ["RateLimiter"]
