from __future__ import annotations

import hashlib
import json
import time
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

from .models import TraceEvent, TraceIds, now_iso
from .validators import SchemaValidator


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


class TraceStore:
    """Manage TRACE files, exports, and replay validation."""

    def __init__(self, traces_dir: Path | str = Path("traces")) -> None:
        self.traces_dir = Path(traces_dir)
        self.traces_dir.mkdir(parents=True, exist_ok=True)
        self.validator = SchemaValidator()

    def list_traces(self) -> List[Dict[str, str]]:
        entries: List[Dict[str, str]] = []
        for trace_file in sorted(self.traces_dir.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True):
            entries.append(
                {
                    "trace_id": trace_file.stem,
                    "path": str(trace_file),
                    "modified": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(trace_file.stat().st_mtime)),
                    "size_bytes": trace_file.stat().st_size,
                }
            )
        return entries

    def export_manifest(self, trace_id: str, output: Path | str) -> Dict:
        trace_path = Path(output)
        trace_file = self.traces_dir / f"{trace_id}.jsonl"
        if not trace_file.exists():
            raise FileNotFoundError(f"Trace file not found for id {trace_id}")

        sha256 = self._hash_file(trace_file)
        events = list(self._load_events(trace_file))
        manifest = {
            "trace_version": "1.0",
            "trace_id": trace_id,
            "trace_file": str(trace_file.resolve()),
            "sha256": sha256,
            "exported_at": now_iso(),
            "event_count": len(events),
            "atlas": events[0].get("atlas") if events else {},
        }
        self.validator.validate("trace.manifest", manifest)
        trace_path.parent.mkdir(parents=True, exist_ok=True)
        trace_path.write_text(json.dumps(manifest, indent=2))
        return manifest

    def replay(self, trace_file: Path | str, fail_fast: bool = False) -> Tuple[int, List[str]]:
        events = self._load_events(Path(trace_file))
        errors: List[str] = []
        count = 0
        for event in events:
            try:
                self.validator.validate("trace.event", event)
            except Exception as exc:  # noqa: BLE001
                errors.append(f"Event {count} failed validation: {exc}")
                if fail_fast:
                    return count, errors
            count += 1
        return count, errors

    def _hash_file(self, path: Path) -> str:
        digest = hashlib.sha256()
        with path.open("rb") as fh:
            for chunk in iter(lambda: fh.read(8192), b""):
                digest.update(chunk)
        return digest.hexdigest()

    def _load_events(self, path: Path) -> Iterable[Dict]:
        with path.open("r", encoding="utf-8") as fh:
            for line in fh:
                if not line.strip():
                    continue
                try:
                    yield json.loads(line)
                except json.JSONDecodeError:
                    continue
