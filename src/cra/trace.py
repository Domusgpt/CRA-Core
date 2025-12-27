from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Dict, Iterable, Optional

from .models import TraceEvent, TraceIds, now_iso


class TraceEmitter:
    def __init__(self, traces_dir: Path | str = Path("traces"), trace_ids: Optional[TraceIds] = None,
                 session_id: Optional[str] = None, atlas: Optional[Dict] = None) -> None:
        self.traces_dir = Path(traces_dir)
        self.traces_dir.mkdir(parents=True, exist_ok=True)
        self.trace_ids = trace_ids or TraceIds()
        self.session_id = session_id or ""
        self.atlas = atlas or {}
        self.trace_file = self.traces_dir / f"{self.trace_ids.trace_id}.jsonl"
        self._write_latest_pointer()

    def _write_latest_pointer(self) -> None:
        latest_file = self.traces_dir / "latest"
        latest_file.write_text(self.trace_ids.trace_id)

    def emit(self, event_type: str, payload: Dict, severity: str = "info", actor: Optional[Dict] = None,
             artifacts: Optional[list] = None) -> Dict:
        event = TraceEvent(
            event_type=event_type,
            payload=payload,
            severity=severity,
            actor=actor or {"type": "runtime", "id": "cra"},
            trace=self.trace_ids,
            session_id=self.session_id,
            atlas=self.atlas,
            artifacts=artifacts or [],
        )
        event_dict = event.to_dict()
        with self.trace_file.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(event_dict) + "\n")
        print(json.dumps(event_dict))
        return event_dict

    def tail(self, follow: bool = False, event_type: Optional[str] = None, severity: Optional[str] = None) -> Iterable[Dict]:
        with self.trace_file.open("r", encoding="utf-8") as fh:
            while True:
                line = fh.readline()
                if not line:
                    if not follow:
                        break
                    time.sleep(0.25)
                    continue
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if event_type and event.get("event_type") != event_type:
                    continue
                if severity and event.get("severity") != severity:
                    continue
                yield event
                if not follow:
                    continue

    @staticmethod
    def latest_trace_id(traces_dir: Path | str = Path("traces")) -> Optional[str]:
        latest_file = Path(traces_dir) / "latest"
        if latest_file.exists():
            return latest_file.read_text().strip()
        traces = sorted(Path(traces_dir).glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True)
        return traces[0].stem if traces else None


def format_trace_metadata() -> Dict[str, str]:
    return {
        "trace_version": "1.0",
        "time": now_iso(),
    }
