"""Policy engine compliance tests.

Verifies that the policy engine correctly enforces governance rules.
"""

from datetime import datetime
from uuid import uuid4

import pytest

from cra.core.policy import (
    DenyPatternRule,
    PolicyContext,
    PolicyDecision,
    PolicyEffect,
    PolicyEngine,
    RateLimitRule,
    RedactionRule,
    RiskTierApprovalRule,
    ScopeRule,
)


class TestPolicyEffects:
    """Tests for policy effect values."""

    def test_policy_effects_are_valid(self):
        """Policy effects must be valid."""
        valid_effects = {
            PolicyEffect.ALLOW,
            PolicyEffect.DENY,
            PolicyEffect.ALLOW_WITH_CONSTRAINTS,
            PolicyEffect.REQUIRE_APPROVAL,
        }
        assert set(PolicyEffect) == valid_effects


class TestScopeRuleCompliance:
    """Tests for scope rule compliance."""

    def test_scope_rule_allows_when_scopes_present(self):
        """Scope rule should allow when all required scopes are present."""
        rule = ScopeRule(
            rule_id="test-scope",
            required_scopes=["read", "write"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=["read", "write", "admin"],
            risk_tier="low",
            goal="Test",
        )
        decision = rule.evaluate(context)
        assert decision is None  # None means rule doesn't apply (allow)

    def test_scope_rule_denies_when_scopes_missing(self):
        """Scope rule should deny when required scopes are missing."""
        rule = ScopeRule(
            rule_id="test-scope",
            required_scopes=["read", "write"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=["read"],  # Missing "write"
            risk_tier="low",
            goal="Test",
        )
        decision = rule.evaluate(context)
        assert decision is not None
        assert decision.effect == PolicyEffect.DENY
        assert "write" in decision.reason


class TestDenyPatternRuleCompliance:
    """Tests for deny pattern rule compliance."""

    def test_deny_rule_blocks_matching_patterns(self):
        """Deny rule should block actions matching patterns."""
        rule = DenyPatternRule(
            rule_id="test-deny",
            patterns=["*.production.*", "rm -rf *"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Deploy to production environment",
        )
        decision = rule.evaluate(context)
        assert decision is not None
        assert decision.effect == PolicyEffect.DENY

    def test_deny_rule_allows_non_matching_patterns(self):
        """Deny rule should allow actions not matching patterns."""
        rule = DenyPatternRule(
            rule_id="test-deny",
            patterns=["*.production.*"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Deploy to staging",
        )
        decision = rule.evaluate(context)
        assert decision is None  # Allowed


class TestRiskTierApprovalRuleCompliance:
    """Tests for risk tier approval rule compliance."""

    def test_high_risk_requires_approval(self):
        """High risk operations should require approval."""
        rule = RiskTierApprovalRule(
            rule_id="test-approval",
            risk_tiers=["high"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="high",
            goal="Test",
        )
        decision = rule.evaluate(context)
        assert decision is not None
        assert decision.effect == PolicyEffect.REQUIRE_APPROVAL
        assert decision.requires_approval is True

    def test_low_risk_does_not_require_approval(self):
        """Low risk operations should not require approval."""
        rule = RiskTierApprovalRule(
            rule_id="test-approval",
            risk_tiers=["high"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Test",
        )
        decision = rule.evaluate(context)
        assert decision is None  # No approval needed


class TestRedactionRuleCompliance:
    """Tests for redaction rule compliance."""

    def test_redaction_rule_identifies_sensitive_fields(self):
        """Redaction rule should identify sensitive fields."""
        rule = RedactionRule(
            rule_id="test-redact",
            patterns=["password", "secret", "token"],
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Test",
            metadata={"password_hash": "xxx", "user_token": "yyy"},
        )
        decision = rule.evaluate(context)
        assert decision is not None
        assert decision.effect == PolicyEffect.ALLOW_WITH_CONSTRAINTS
        assert len(decision.redactions) > 0


class TestPolicyEngineCompliance:
    """Tests for policy engine compliance."""

    @pytest.fixture
    def engine(self):
        """Create a policy engine."""
        return PolicyEngine()

    def test_engine_has_default_rules(self, engine):
        """Engine should have default rules."""
        assert len(engine._rules) > 0

    def test_engine_evaluates_all_rules(self, engine):
        """Engine should evaluate all rules."""
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Safe operation",
        )
        decision = engine.evaluate(context)
        assert decision is not None
        assert isinstance(decision, PolicyDecision)

    def test_deny_takes_precedence(self, engine):
        """DENY should take precedence over other effects."""
        # Add a deny rule that matches
        engine.add_rule(
            DenyPatternRule(
                rule_id="always-deny",
                patterns=["*"],
            )
        )
        context = PolicyContext(
            session_id=uuid4(),
            principal_type="user",
            principal_id="test-user",
            scopes=[],
            risk_tier="low",
            goal="Something",
        )
        decision = engine.evaluate(context)
        assert decision.effect == PolicyEffect.DENY

    def test_dangerous_patterns_are_blocked(self, engine):
        """Dangerous patterns should be blocked by default."""
        dangerous_goals = [
            "rm -rf /",
            "DROP TABLE users",
            "DELETE FROM database",
        ]
        for goal in dangerous_goals:
            context = PolicyContext(
                session_id=uuid4(),
                principal_type="user",
                principal_id="test-user",
                scopes=[],
                risk_tier="low",
                goal=goal,
            )
            decision = engine.evaluate(context)
            assert decision.effect == PolicyEffect.DENY, f"Should deny: {goal}"

    def test_add_and_remove_rules(self, engine):
        """Rules can be added and removed."""
        initial_count = len(engine._rules)

        rule = ScopeRule(rule_id="custom-scope", required_scopes=["custom"])
        engine.add_rule(rule)
        assert len(engine._rules) == initial_count + 1

        removed = engine.remove_rule("custom-scope")
        assert removed is True
        assert len(engine._rules) == initial_count

    def test_remove_nonexistent_rule_returns_false(self, engine):
        """Removing nonexistent rule returns False."""
        removed = engine.remove_rule("nonexistent-rule")
        assert removed is False


class TestPolicyDecisionCompliance:
    """Tests for policy decision structure compliance."""

    def test_decision_has_effect(self):
        """Decisions must have an effect."""
        decision = PolicyDecision(effect=PolicyEffect.ALLOW)
        assert decision.effect is not None

    def test_decision_can_have_violations(self):
        """Decisions can have violations."""
        from cra.core.policy import PolicyViolation

        decision = PolicyDecision(
            effect=PolicyEffect.DENY,
            violations=[
                PolicyViolation(
                    rule_id="test",
                    reason="Test violation",
                )
            ],
        )
        assert len(decision.violations) == 1

    def test_decision_can_have_constraints(self):
        """Decisions can have constraints."""
        decision = PolicyDecision(
            effect=PolicyEffect.ALLOW_WITH_CONSTRAINTS,
            constraints={"max_retries": 3},
        )
        assert decision.constraints["max_retries"] == 3

    def test_decision_can_have_redactions(self):
        """Decisions can have redactions."""
        decision = PolicyDecision(
            effect=PolicyEffect.ALLOW_WITH_CONSTRAINTS,
            redactions=["password", "token"],
        )
        assert len(decision.redactions) == 2
