from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

ISOFORMAT = "%Y-%m-%dT%H:%M:%S.%fZ"


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime(ISOFORMAT)


@dataclass
class TraceIds:
    trace_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    span_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    parent_span_id: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "trace_id": self.trace_id,
            "span_id": self.span_id,
            "parent_span_id": self.parent_span_id,
        }


@dataclass
class Session:
    session_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    principal_type: str = "user"
    principal_id: str = "unknown"
    scopes: List[str] = field(default_factory=list)
    expires_at: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "session_id": self.session_id,
            "principal": {"type": self.principal_type, "id": self.principal_id},
            "scopes": self.scopes,
            "expires_at": self.expires_at,
        }


@dataclass
class CarpRequest:
    goal: str
    target_platforms: List[str]
    risk_tier: str = "low"
    constraints: List[str] = field(default_factory=list)
    environment: Dict[str, Any] = field(default_factory=dict)
    preferences: Dict[str, Any] = field(default_factory=dict)
    session: Session = field(default_factory=Session)
    atlas: Dict[str, Any] = field(default_factory=dict)
    trace: TraceIds = field(default_factory=TraceIds)

    def to_envelope(self) -> Dict[str, Any]:
        return {
            "carp_version": "1.0",
            "type": "carp.request",
            "id": str(uuid.uuid4()),
            "time": now_iso(),
            "session": self.session.to_dict(),
            "atlas": self.atlas,
            "payload": {
                "operation": "resolve",
                "task": {
                    "goal": self.goal,
                    "constraints": self.constraints,
                    "target_platforms": self.target_platforms,
                    "risk_tier": self.risk_tier,
                    "inputs": [],
                },
                "environment": self.environment,
                "preferences": self.preferences,
            },
            "trace": self.trace.to_dict(),
        }


@dataclass
class ContextBlock:
    block_id: str
    purpose: str
    ttl_seconds: int
    content_type: str
    content: Any
    redactions: List[Dict[str, str]] = field(default_factory=list)
    source_evidence: List[Dict[str, str]] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "block_id": self.block_id,
            "purpose": self.purpose,
            "ttl_seconds": self.ttl_seconds,
            "content_type": self.content_type,
            "content": self.content,
            "redactions": self.redactions,
            "source_evidence": self.source_evidence,
        }


@dataclass
class AllowedAction:
    action_id: str
    kind: str
    adapter: str
    schema: Dict[str, Any]
    constraints: List[Dict[str, Any]] = field(default_factory=list)
    requires_approval: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "action_id": self.action_id,
            "kind": self.kind,
            "adapter": self.adapter,
            "schema": {"json_schema": self.schema},
            "constraints": self.constraints,
            "requires_approval": self.requires_approval,
        }


@dataclass
class Resolution:
    resolution_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    confidence: float = 0.62
    context_blocks: List[ContextBlock] = field(default_factory=list)
    allowed_actions: List[AllowedAction] = field(default_factory=list)
    denylist: List[Dict[str, Any]] = field(default_factory=list)
    merge_rules: Dict[str, Any] = field(default_factory=lambda: {"conflict": "fail"})
    next_steps: List[Dict[str, Any]] = field(default_factory=list)

    def to_payload(self) -> Dict[str, Any]:
        return {
            "operation": "resolve",
            "resolution": {
                "resolution_id": self.resolution_id,
                "confidence": self.confidence,
                "context_blocks": [c.to_dict() for c in self.context_blocks],
                "allowed_actions": [a.to_dict() for a in self.allowed_actions],
                "denylist": self.denylist,
                "merge_rules": self.merge_rules,
                "next_steps": self.next_steps,
            },
        }

    def to_response(self, session: Session, atlas: Dict[str, Any], trace: TraceIds) -> Dict[str, Any]:
        return {
            "carp_version": "1.0",
            "type": "carp.response",
            "id": str(uuid.uuid4()),
            "time": now_iso(),
            "session": session.to_dict(),
            "atlas": atlas,
            "payload": self.to_payload(),
            "trace": trace.to_dict(),
        }


@dataclass
class TraceEvent:
    event_type: str
    payload: Dict[str, Any]
    severity: str = "info"
    actor: Dict[str, str] = field(default_factory=lambda: {"type": "runtime", "id": "cra"})
    trace: TraceIds = field(default_factory=TraceIds)
    session_id: str = ""
    atlas: Optional[Dict[str, Any]] = None
    artifacts: List[Dict[str, Any]] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "trace_version": "1.0",
            "event_type": self.event_type,
            "time": now_iso(),
            "trace": self.trace.to_dict(),
            "session_id": self.session_id,
            "atlas": self.atlas or {},
            "actor": self.actor,
            "severity": self.severity,
            "payload": self.payload,
            "artifacts": self.artifacts,
        }
