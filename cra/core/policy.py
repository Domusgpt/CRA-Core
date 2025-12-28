"""Policy engine for CRA.

Provides governance and enforcement for CARP resolutions:
- Scope validation
- Deny rules
- Redaction
- Approval gates
- Rate limiting
"""

import re
from abc import ABC, abstractmethod
from datetime import datetime
from enum import Enum
from typing import Any
from uuid import UUID

from pydantic import BaseModel, Field


class PolicyEffect(str, Enum):
    """Effect of a policy decision."""

    ALLOW = "allow"
    DENY = "deny"
    ALLOW_WITH_CONSTRAINTS = "allow_with_constraints"
    REQUIRE_APPROVAL = "require_approval"


class PolicyViolation(BaseModel):
    """A policy violation."""

    rule_id: str
    reason: str
    severity: str = "error"
    details: dict[str, Any] = Field(default_factory=dict)


class PolicyDecision(BaseModel):
    """Result of policy evaluation."""

    effect: PolicyEffect
    rule_id: str | None = None
    reason: str = ""
    violations: list[PolicyViolation] = Field(default_factory=list)
    constraints: dict[str, Any] = Field(default_factory=dict)
    redactions: list[str] = Field(default_factory=list)
    requires_approval: bool = False
    approval_reason: str | None = None


class PolicyContext(BaseModel):
    """Context for policy evaluation."""

    session_id: UUID
    principal_type: str
    principal_id: str
    scopes: list[str]
    risk_tier: str
    goal: str
    action_id: str | None = None
    resource: str | None = None
    timestamp: datetime = Field(default_factory=datetime.utcnow)
    metadata: dict[str, Any] = Field(default_factory=dict)


class PolicyRule(ABC):
    """Base class for policy rules."""

    def __init__(self, rule_id: str, description: str = "") -> None:
        self.rule_id = rule_id
        self.description = description

    @abstractmethod
    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Evaluate the rule against a context.

        Args:
            context: The policy context

        Returns:
            PolicyDecision if rule applies, None otherwise
        """
        pass


class ScopeRule(PolicyRule):
    """Validates that required scopes are present."""

    def __init__(
        self,
        rule_id: str,
        required_scopes: list[str],
        description: str = "",
    ) -> None:
        super().__init__(rule_id, description)
        self.required_scopes = required_scopes

    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Check if all required scopes are present."""
        missing = [s for s in self.required_scopes if s not in context.scopes]

        if missing:
            return PolicyDecision(
                effect=PolicyEffect.DENY,
                rule_id=self.rule_id,
                reason=f"Missing required scopes: {', '.join(missing)}",
                violations=[
                    PolicyViolation(
                        rule_id=self.rule_id,
                        reason=f"Missing scope: {scope}",
                        details={"scope": scope},
                    )
                    for scope in missing
                ],
            )
        return None


class DenyPatternRule(PolicyRule):
    """Denies actions matching specific patterns."""

    def __init__(
        self,
        rule_id: str,
        patterns: list[str],
        description: str = "",
    ) -> None:
        super().__init__(rule_id, description)
        self.patterns = [re.compile(self._glob_to_regex(p), re.IGNORECASE) for p in patterns]
        self.pattern_strings = patterns

    def _glob_to_regex(self, pattern: str) -> str:
        """Convert glob pattern to regex."""
        # Escape special chars except * and ?
        escaped = re.escape(pattern)
        # Convert * to .* and ? to .
        escaped = escaped.replace(r"\*", ".*").replace(r"\?", ".")
        return f"^{escaped}$"

    def _normalize_target(self, target: str) -> str:
        """Normalize human-readable targets for consistent pattern matching.

        This makes phrases such as "Deploy to production environment" compatible
        with dot-delimited deny patterns (e.g. "*.production.*") by replacing
        whitespace and punctuation with dots, collapsing repeats, and lowering
        case.
        """

        normalized = re.sub(r"[^a-zA-Z0-9]+", ".", target.lower())
        return re.sub(r"\.+", ".", normalized).strip(".")

    def _candidate_targets(self, target: str) -> list[str]:
        """Return candidate strings (original + normalized when useful)."""

        candidates = [target]
        # Only add a normalized candidate when the target contains human-friendly
        # separators rather than dot-delimited identifiers to avoid surprising
        # over-matching on already-scoped action/resource IDs.
        if re.search(r"[^a-zA-Z0-9_.-]", target):
            normalized = self._normalize_target(target)
            if normalized and normalized not in candidates:
                candidates.append(normalized)
        return candidates

    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Check if action or resource matches deny patterns."""
        targets = [context.action_id, context.resource, context.goal]
        targets = [t for t in targets if t]

        for target in targets:
            for candidate in self._candidate_targets(target):
                for i, pattern in enumerate(self.patterns):
                    if pattern.match(candidate):
                        details = {
                            "pattern": self.pattern_strings[i],
                            "matched": target,
                        }
                        if candidate != target:
                            details["normalized_match"] = candidate

                        return PolicyDecision(
                            effect=PolicyEffect.DENY,
                            rule_id=self.rule_id,
                            reason=f"Denied by pattern: {self.pattern_strings[i]}",
                            violations=[
                                PolicyViolation(
                                    rule_id=self.rule_id,
                                    reason="Matched deny pattern",
                                    details=details,
                                )
                            ],
                        )
        return None


class RiskTierApprovalRule(PolicyRule):
    """Requires approval for high-risk operations."""

    def __init__(
        self,
        rule_id: str,
        risk_tiers: list[str],
        description: str = "",
    ) -> None:
        super().__init__(rule_id, description)
        self.risk_tiers = risk_tiers

    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Check if risk tier requires approval."""
        if context.risk_tier in self.risk_tiers:
            return PolicyDecision(
                effect=PolicyEffect.REQUIRE_APPROVAL,
                rule_id=self.rule_id,
                reason=f"Risk tier '{context.risk_tier}' requires approval",
                requires_approval=True,
                approval_reason=f"High-risk operation: {context.goal}",
            )
        return None


class RateLimitRule(PolicyRule):
    """Enforces rate limits on actions."""

    def __init__(
        self,
        rule_id: str,
        max_requests: int,
        window_seconds: int,
        description: str = "",
    ) -> None:
        super().__init__(rule_id, description)
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        # In-memory tracking (would be Redis/DB in production)
        self._requests: dict[str, list[datetime]] = {}

    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Check if rate limit is exceeded."""
        key = f"{context.principal_id}:{context.action_id or 'any'}"
        now = context.timestamp
        cutoff = now.timestamp() - self.window_seconds

        # Get recent requests
        if key not in self._requests:
            self._requests[key] = []

        # Filter to window
        self._requests[key] = [
            t for t in self._requests[key] if t.timestamp() > cutoff
        ]

        if len(self._requests[key]) >= self.max_requests:
            return PolicyDecision(
                effect=PolicyEffect.DENY,
                rule_id=self.rule_id,
                reason=f"Rate limit exceeded: {self.max_requests} requests per {self.window_seconds}s",
                violations=[
                    PolicyViolation(
                        rule_id=self.rule_id,
                        reason="Rate limit exceeded",
                        details={
                            "limit": self.max_requests,
                            "window_seconds": self.window_seconds,
                            "current_count": len(self._requests[key]),
                        },
                    )
                ],
            )

        # Record this request
        self._requests[key].append(now)
        return None


class RedactionRule(PolicyRule):
    """Redacts sensitive fields from context."""

    def __init__(
        self,
        rule_id: str,
        patterns: list[str],
        description: str = "",
    ) -> None:
        super().__init__(rule_id, description)
        self.patterns = patterns

    def evaluate(self, context: PolicyContext) -> PolicyDecision | None:
        """Return redaction requirements."""
        # Always return redactions if patterns match metadata
        matching_redactions = []
        for pattern in self.patterns:
            # Simple field name matching
            if any(pattern.lower() in k.lower() for k in context.metadata.keys()):
                matching_redactions.append(pattern)

        if matching_redactions:
            return PolicyDecision(
                effect=PolicyEffect.ALLOW_WITH_CONSTRAINTS,
                rule_id=self.rule_id,
                reason="Fields require redaction",
                redactions=matching_redactions,
            )
        return None


class PolicyEngine:
    """Evaluates policies for CARP resolutions.

    The policy engine applies rules in order and returns
    the most restrictive decision.
    """

    def __init__(self) -> None:
        """Initialize the policy engine."""
        self._rules: list[PolicyRule] = []
        self._setup_default_rules()

    def _setup_default_rules(self) -> None:
        """Set up default policy rules."""
        # Deny dangerous patterns
        self.add_rule(
            DenyPatternRule(
                rule_id="deny-dangerous-commands",
                patterns=[
                    "rm -rf *",
                    "rm -rf /",
                    "dd if=*",
                    "mkfs.*",
                    ":(){ :|:& };:",
                    "*.production.*",
                    "DROP TABLE*",
                    "DELETE FROM*",
                ],
                description="Deny dangerous system commands",
            )
        )

        # Require approval for high-risk
        self.add_rule(
            RiskTierApprovalRule(
                rule_id="high-risk-approval",
                risk_tiers=["high"],
                description="Require approval for high-risk operations",
            )
        )

        # Redact sensitive fields
        self.add_rule(
            RedactionRule(
                rule_id="redact-secrets",
                patterns=["password", "secret", "token", "api_key", "credential"],
                description="Redact sensitive fields",
            )
        )

    def add_rule(self, rule: PolicyRule) -> None:
        """Add a policy rule.

        Args:
            rule: The rule to add
        """
        self._rules.append(rule)

    def remove_rule(self, rule_id: str) -> bool:
        """Remove a policy rule by ID.

        Args:
            rule_id: The rule ID to remove

        Returns:
            True if rule was removed, False if not found
        """
        initial_count = len(self._rules)
        self._rules = [r for r in self._rules if r.rule_id != rule_id]
        return len(self._rules) < initial_count

    def evaluate(self, context: PolicyContext) -> PolicyDecision:
        """Evaluate all policies for a context.

        Args:
            context: The policy context

        Returns:
            The most restrictive policy decision
        """
        all_violations: list[PolicyViolation] = []
        all_redactions: list[str] = []
        all_constraints: dict[str, Any] = {}
        requires_approval = False
        approval_reason = None

        # Track most restrictive effect
        final_effect = PolicyEffect.ALLOW
        final_rule_id = None
        final_reason = "Allowed by default policy"

        for rule in self._rules:
            decision = rule.evaluate(context)
            if decision is None:
                continue

            # Collect violations
            all_violations.extend(decision.violations)
            all_redactions.extend(decision.redactions)
            all_constraints.update(decision.constraints)

            if decision.requires_approval:
                requires_approval = True
                approval_reason = decision.approval_reason

            # DENY is most restrictive
            if decision.effect == PolicyEffect.DENY:
                return PolicyDecision(
                    effect=PolicyEffect.DENY,
                    rule_id=decision.rule_id,
                    reason=decision.reason,
                    violations=all_violations,
                )

            # Track REQUIRE_APPROVAL
            if decision.effect == PolicyEffect.REQUIRE_APPROVAL:
                if final_effect != PolicyEffect.DENY:
                    final_effect = PolicyEffect.REQUIRE_APPROVAL
                    final_rule_id = decision.rule_id
                    final_reason = decision.reason

            # Track ALLOW_WITH_CONSTRAINTS
            if decision.effect == PolicyEffect.ALLOW_WITH_CONSTRAINTS:
                if final_effect == PolicyEffect.ALLOW:
                    final_effect = PolicyEffect.ALLOW_WITH_CONSTRAINTS
                    final_rule_id = decision.rule_id
                    final_reason = decision.reason

        return PolicyDecision(
            effect=final_effect,
            rule_id=final_rule_id,
            reason=final_reason,
            violations=all_violations,
            constraints=all_constraints,
            redactions=list(set(all_redactions)),
            requires_approval=requires_approval,
            approval_reason=approval_reason,
        )


# Global policy engine instance
_policy_engine: PolicyEngine | None = None


def get_policy_engine() -> PolicyEngine:
    """Get the global policy engine instance."""
    global _policy_engine
    if _policy_engine is None:
        _policy_engine = PolicyEngine()
    return _policy_engine
