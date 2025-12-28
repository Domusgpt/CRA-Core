from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional

from jsonschema import ValidationError

from .models import AllowedAction, CarpRequest, ContextBlock, Resolution, Session, TraceIds
from .auth import AuthManager
from .license import LicenseManager
from .policy import PolicyDecision, PolicyEngine
from .ratelimit import RateLimiter
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
        self._validate_adapters()

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
                constraints = list(action.get("constraints", []))
                risk = action.get("risk", "low")
                requires_approval = action.get("requires_approval") or adapter_name.startswith(
                    "google"
                )
                requires_approval = bool(requires_approval or risk == "high")
                required_scopes = action.get("required_scopes", [])
                if required_scopes and not any(
                    c.get("type") == "scope" for c in constraints
                ):
                    constraints.append({"type": "scope", "value": required_scopes})
                rate_limit = action.get("rate_limit") or adapter.get("rate_limit")
                if rate_limit:
                    constraints.append(
                        {
                            "type": "rate_limit",
                            "value": rate_limit.get("limit"),
                            "window_seconds": rate_limit.get("window_seconds", 3600),
                        }
                    )
                actions.append(
                    AllowedAction(
                        action_id=f"{adapter_name}.{action.get('name', idx)}",
                        kind="tool_call",
                        adapter=adapter_name,
                        schema=action.get("schema", {}),
                        constraints=constraints,
                        requires_approval=requires_approval,
                        rate_limit=rate_limit,
                        approval_policy="required" if requires_approval else "auto",
                        required_scopes=required_scopes,
                    )
                )
        return actions

    def _validate_adapters(self) -> None:
        adapters = self.manifest.get("adapters", {})
        if not isinstance(adapters, dict):
            raise ValueError("Atlas adapters must be an object keyed by platform")

        declared_platforms = set(self.manifest.get("platform_adapters", []))
        missing = [p for p in declared_platforms if p not in adapters]
        if missing:
            raise ValueError(f"Adapters missing for platforms: {', '.join(missing)}")

        for adapter_name, adapter in adapters.items():
            actions = adapter.get("actions", [])
            if not isinstance(actions, list) or not actions:
                raise ValueError(f"Adapter {adapter_name} must declare at least one action")
            for action in actions:
                if "name" not in action:
                    raise ValueError(f"Adapter {adapter_name} action missing name")
                if "schema" not in action:
                    raise ValueError(
                        f"Adapter {adapter_name} action '{action.get('name', 'unknown')}' missing schema"
                    )


class CRARuntime:
    def __init__(self, atlas_path: Path | str = Path("atlas/reference")) -> None:
        self.atlas_loader = AtlasLoader(atlas_path)
        self.validator = SchemaValidator()
        self.policy_engine = PolicyEngine(self.atlas_loader.manifest)
        self.license_manager = LicenseManager()
        self.rate_limiter = RateLimiter()
        self.auth_manager = AuthManager()

    def resolve(self, goal: str, target_platforms: Optional[List[str]] = None, risk_tier: str = "low",
                constraints: Optional[List[str]] = None, environment: Optional[Dict] = None,
                preferences: Optional[Dict] = None, session: Optional[Session] = None,
                trace_ids: Optional[TraceIds] = None, token: Optional[str] = None) -> Dict:
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

        trace_emitter = TraceEmitter(
            trace_ids=trace_ids, session_id=session.session_id, atlas=request.atlas
        )
        allowed, reason = self.license_manager.check(self.atlas_loader.manifest)
        if not allowed:
            response = self._build_denied_response(
                PolicyDecision(allowed=False, reasons=[reason]),
                session=session,
                atlas=request.atlas,
                trace=trace_ids,
            )
            trace_emitter.emit(
                "trace.carp.resolve.policy.denied",
                {"reason": reason},
                severity="warn",
            )
            trace_emitter.emit("trace.carp.resolve.returned", response)
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return response

        auth_allowed, auth_reason = self.auth_manager.validate(
            session=session, token=token, required_scopes=[]
        )
        if not auth_allowed:
            response = self._build_denied_response(
                PolicyDecision(allowed=False, reasons=[auth_reason]),
                session=session,
                atlas=request.atlas,
                trace=trace_ids,
            )
            trace_emitter.emit(
                "trace.carp.resolve.policy.denied",
                {"reason": auth_reason, "policy": "auth"},
                severity="error",
            )
            trace_emitter.emit("trace.carp.resolve.returned", response)
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return response

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

    def invoke_action(
        self,
        action_id: str,
        payload: Optional[Dict] = None,
        auto_approve: bool = False,
        session: Optional[Session] = None,
        trace_ids: Optional[TraceIds] = None,
        token: Optional[str] = None,
    ) -> Dict:
        session = session or Session()
        trace_ids = trace_ids or TraceIds()
        payload = payload or {}

        trace_emitter = TraceEmitter(
            trace_ids=trace_ids,
            session_id=session.session_id,
            atlas={
                "id": self.atlas_loader.manifest.get("id"),
                "version": self.atlas_loader.manifest.get("version"),
            },
        )
        allowed, reason = self.license_manager.check(self.atlas_loader.manifest)
        trace_emitter.emit("trace.session.started", {"session": session.to_dict()})

        if not allowed:
            trace_emitter.emit(
                "trace.action.policy.denied",
                {"action_id": action_id, "reason": reason, "policy": "license"},
                severity="warn",
            )
            trace_emitter.emit(
                "trace.action.failed",
                {"action_id": action_id, "reason": reason},
                severity="warn",
            )
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return {"action_id": action_id, "status": "denied", "reason": reason}

        allowed_actions = {a.action_id: a for a in self.atlas_loader.allowed_actions()}
        action = allowed_actions.get(action_id)
        if not action:
            trace_emitter.emit(
                "trace.action.failed",
                {"action_id": action_id, "reason": "unknown action"},
                severity="error",
            )
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return {"action_id": action_id, "status": "failed", "reason": "unknown action"}

        auth_allowed, auth_reason = self.auth_manager.validate(
            session=session, token=token, required_scopes=action.required_scopes
        )
        if not auth_allowed:
            trace_emitter.emit(
                "trace.action.policy.denied",
                {
                    "action_id": action_id,
                    "reason": auth_reason,
                    "policy": "auth",
                    "required_scopes": action.required_scopes,
                },
                severity="error",
            )
            trace_emitter.emit(
                "trace.action.failed",
                {"action_id": action_id, "reason": auth_reason},
                severity="error",
            )
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return {"action_id": action_id, "status": "denied", "reason": auth_reason}

        if action.requires_approval and not auto_approve:
            trace_emitter.emit(
                "trace.action.pending_approval",
                {"action_id": action_id, "reason": "approval required"},
                severity="warn",
            )
            trace_emitter.emit(
                "trace.action.failed",
                {"action_id": action_id, "reason": "approval required"},
                severity="warn",
            )
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return {
                "action_id": action_id,
                "status": "pending_approval",
                "reason": "approval required",
            }

        rate_limit_value = 10
        window_seconds = 3600
        if action.rate_limit:
            rate_limit_value = action.rate_limit.get("limit", rate_limit_value)
            window_seconds = action.rate_limit.get("window_seconds", window_seconds)
        else:
            for constraint in action.constraints:
                if constraint.get("type") == "rate_limit":
                    try:
                        rate_limit_value = int(constraint.get("value", rate_limit_value))
                        window_seconds = int(
                            constraint.get("window_seconds", window_seconds)
                        )
                    except (TypeError, ValueError):
                        rate_limit_value = rate_limit_value
                        window_seconds = window_seconds

        allowed_rate, rate_reason = self.rate_limiter.check(
            action_id, limit=rate_limit_value, window_seconds=window_seconds
        )
        if not allowed_rate:
            trace_emitter.emit(
                "trace.action.rate_limited",
                {
                    "action_id": action_id,
                    "reason": rate_reason,
                    "window_seconds": window_seconds,
                },
                severity="warn",
            )
            trace_emitter.emit(
                "trace.action.failed",
                {"action_id": action_id, "reason": rate_reason},
                severity="warn",
            )
            trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
            return {"action_id": action_id, "status": "rate_limited", "reason": rate_reason}

        trace_emitter.emit(
            "trace.action.granted",
            {
                "action_id": action_id,
                "approval": auto_approve or not action.requires_approval,
                "rate_limit": {
                    "limit": rate_limit_value,
                    "window_seconds": window_seconds,
                },
                "scopes": action.required_scopes,
            },
        )
        trace_emitter.emit("trace.action.invoked", {"action_id": action_id, "payload": payload})

        result_payload = {
            "action_id": action_id,
            "status": "completed",
            "echo": payload,
            "rate_window": rate_reason,
        }
        trace_emitter.emit("trace.action.completed", result_payload)
        trace_emitter.emit("trace.session.ended", {"session": session.to_dict()})
        return result_payload

    def _build_resolution(self, risk_tier: str) -> Resolution:
        context_blocks = self.atlas_loader.context_blocks()
        allowed_actions = self.atlas_loader.allowed_actions()

        license_model = self.atlas_loader.manifest.get("licensing", {}).get("model", "free")

        for action in allowed_actions:
            if risk_tier == "high":
                action.requires_approval = True
                action.constraints.append({"type": "approval", "value": "required"})
            action.constraints.append({"type": "risk_tier", "value": risk_tier})
            if license_model != "free":
                action.constraints.append({"type": "license", "value": license_model})

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
