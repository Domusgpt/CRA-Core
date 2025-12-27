from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List


RISK_ORDER = ["low", "medium", "high"]


@dataclass
class PolicyDecision:
    allowed: bool
    reasons: List[str]

    def to_payload(self) -> Dict:
        return {"allowed": self.allowed, "reasons": self.reasons}


class PolicyEngine:
    """Very small policy gate based on Atlas capability and requested risk tier."""

    def __init__(self, atlas_manifest: Dict) -> None:
        self.manifest = atlas_manifest

    def evaluate(self, requested_risk: str, target_platforms: List[str]) -> PolicyDecision:
        reasons: List[str] = []
        allowed = True

        atlas_platforms = self.manifest.get("platform_adapters", [])
        unsupported = [p for p in target_platforms if p not in atlas_platforms]
        if unsupported:
            allowed = False
            reasons.append(f"Unsupported platform adapters: {', '.join(unsupported)}")

        capability_risk = self._capability_risk_floor()
        if RISK_ORDER.index(requested_risk) > RISK_ORDER.index(capability_risk):
            allowed = False
            reasons.append(
                f"Requested risk tier '{requested_risk}' exceeds capability floor '{capability_risk}'"
            )

        return PolicyDecision(allowed=allowed, reasons=reasons)

    def _capability_risk_floor(self) -> str:
        capabilities = self.manifest.get("capabilities", [])
        if not capabilities:
            return "low"
        risk_values = [c.get("risk_tier", "low") for c in capabilities]
        # pick max risk_tier allowed by capabilities
        risk_values = [r for r in risk_values if r in RISK_ORDER]
        if not risk_values:
            return "low"
        return sorted(risk_values, key=lambda r: RISK_ORDER.index(r))[-1]


__all__ = ["PolicyEngine", "PolicyDecision", "RISK_ORDER"]
