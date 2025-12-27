from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional

from jsonschema import ValidationError

from .models import AllowedAction, CarpRequest, ContextBlock, Resolution, Session, TraceIds
from .policy import PolicyDecision, PolicyEngine
from .trace import TraceEmitter
from .validators import SchemaValidator


class AtlasLoader:
    def __init__(self, atlas_path: Path | str) -> None:
        self.atlas_root = Path(atlas_path)
        self.manifest_path = self.atlas_root / "atlas.json"
        if not self.manifest_path.exists():
            raise FileNotFoundError(f"Atlas manifest not found at {self.manifest_path}")
        self.manifest = json.loads(self.manifest_path.read_text())
        SchemaValidator().validate("atlas", self.manifest)

    def context_blocks(self) -> List[ContextBlock]:
        compact_path = self.atlas_root / "context" / "compact.md"
        content = compact_path.read_text() if compact_path.exists() else ""
        return [
            ContextBlock(
                block_id="context.compact",
                purpose="baseline",
                ttl_seconds=3600,
                content_type="text/markdown",
                content=content,
                source_evidence=[{"type": "doc", "ref": str(compact_path), "hash": ""}]
                if content
                else [],
            )
        ]

    def allowed_actions(self) -> List[AllowedAction]:
        actions: List[AllowedAction] = []
        adapters = self.manifest.get("adapters", {})
        for adapter_name, adapter in adapters.items():
            for idx, action in enumerate(adapter.get("actions", [])):
                actions.append(
                    AllowedAction(
                        action_id=f"{adapter_name}.{idx}",
                        kind="tool_call",
                        adapter=adapter_name,
                        schema=action.get("schema", {}),
                        requires_approval=adapter_name.startswith("google")
                        or action.get("risk", "low") == "high",
                    )
                )
        return actions


class CRARuntime:
    def __init__(self, atlas_path: Path | str = Path("atlas/reference")) -> None:
        self.atlas_loader = AtlasLoader(atlas_path)
        self.validator = SchemaValidator()
        self.policy_engine = PolicyEngine(self.atlas_loader.manifest)

    def resolve(self, goal: str, target_platforms: Optional[List[str]] = None, risk_tier: str = "low",
                constraints: Optional[List[str]] = None, environment: Optional[Dict] = None,
                preferences: Optional[Dict] = None, session: Optional[Session] = None,
                trace_ids: Optional[TraceIds] = None) -> Dict:
        session = session or Session()
        trace_ids = trace_ids or TraceIds()
        constraints = constraints or []
        environment = environment or {}
        preferences = preferences or {}
        target_platforms = target_platforms or self.atlas_loader.manifest.get("platform_adapters", [])

        request = CarpRequest(
            goal=goal,
            target_platforms=target_platforms,
            risk_tier=risk_tier,
            constraints=constraints,
            environment=environment,
            preferences=preferences,
            session=session,
            atlas={"id": self.atlas_loader.manifest.get("id"), "version": self.atlas_loader.manifest.get("version")},
            trace=trace_ids,
        )

        trace_emitter = TraceEmitter(trace_ids=trace_ids, session_id=session.session_id,
                                     atlas=request.atlas)
        trace_emitter.emit("trace.session.started", {"session": session.to_dict()})

        try:
            request_envelope = request.to_envelope()
            self.validator.validate("carp.request", request_envelope)
            trace_emitter.emit("trace.carp.resolve.requested", request_envelope)
        except ValidationError as exc:
            trace_emitter.emit(
                "trace.validation.error",
                {"stage": "carp.request", "message": str(exc)},
                severity="error",
            )
            raise

        policy_decision = self.policy_engine.evaluate(risk_tier, target_platforms)
        if not policy_decision.allowed:
            response = self._build_denied_response(
                policy_decision, session=session, atlas=request.atlas, trace=trace_ids
            )
            trace_emitter.emit(
                "trace.carp.resolve.policy.denied",
                policy_decision.to_payload(),
                severity="warn",
            )
            trace_emitter.emit("trace.carp.resolve.returned", response)
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return response

        try:
            resolution = self._build_resolution(risk_tier)
            response = resolution.to_response(session=session, atlas=request.atlas, trace=trace_ids)
            self.validator.validate("carp.response", response)
            trace_emitter.emit("trace.carp.resolve.returned", response)
        except ValidationError as exc:
            trace_emitter.emit(
                "trace.validation.error",
                {"stage": "carp.response", "message": str(exc)},
                severity="error",
            )
            raise
        except Exception as exc:  # noqa: BLE001
            trace_emitter.emit(
                "trace.runtime.error", {"message": str(exc)}, severity="error"
            )
            raise
        finally:
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
        return response

    def _build_resolution(self, risk_tier: str) -> Resolution:
        context_blocks = self.atlas_loader.context_blocks()
        allowed_actions = self.atlas_loader.allowed_actions()

        for action in allowed_actions:
            if risk_tier == "high":
                action.requires_approval = True
                action.constraints.append({"type": "approval", "value": "required"})
            action.constraints.append({"type": "risk_tier", "value": risk_tier})

        denylist = []
        if risk_tier in {"medium", "high"}:
            denylist.append({"pattern": "rm -rf", "reason": "dangerous shell"})

        next_steps = [
            {"step": "invoke_adapter", "expected_artifacts": ["action.plan"]},
            {"step": "stream_trace", "expected_artifacts": ["trace.jsonl"]},
        ]

        return Resolution(
            context_blocks=context_blocks,
            allowed_actions=allowed_actions,
            denylist=denylist,
            next_steps=next_steps,
        )

    def _build_denied_response(self, decision: PolicyDecision, session: Session, atlas: Dict, trace: TraceIds) -> Dict:
        resolution = Resolution(
            context_blocks=[],
            allowed_actions=[],
            denylist=[{"pattern": "*", "reason": reason} for reason in decision.reasons],
            confidence=0.0,
            next_steps=[{"step": "review_policy", "expected_artifacts": []}],
        )
        return resolution.to_response(session=session, atlas=atlas, trace=trace)
