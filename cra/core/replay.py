"""TRACE replay and golden trace support.

Provides deterministic replay of TRACE logs for:
- Regression testing
- Compliance audits
- Debugging
"""

import json
import re
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any
from uuid import UUID

from pydantic import BaseModel, Field

from cra.core.trace import TraceEvent


class NondeterminismRuleType(str, Enum):
    """Type of nondeterminism handling."""

    IGNORE = "ignore"  # Completely ignore this field
    NORMALIZE = "normalize"  # Normalize to a fixed value
    MASK = "mask"  # Replace with a mask pattern
    PATTERN = "pattern"  # Must match a regex pattern


class NondeterminismRule(BaseModel):
    """Rule for handling nondeterministic fields during replay."""

    field: str  # JSONPath-like selector (e.g., "*.time", "trace.span_id")
    rule: NondeterminismRuleType
    value: str | None = None  # For normalize/mask/pattern


class ReplayArtifact(BaseModel):
    """An artifact in a replay manifest."""

    name: str
    uri: str
    sha256: str
    content_type: str


class ReplayManifest(BaseModel):
    """Manifest for replaying a trace.

    The manifest specifies:
    - The trace to replay
    - Expected events
    - Rules for handling nondeterminism
    """

    manifest_version: str = "1.0"
    trace_id: UUID
    name: str = ""
    description: str = ""
    created_at: datetime = Field(default_factory=datetime.utcnow)
    artifacts: list[ReplayArtifact] = Field(default_factory=list)
    nondeterminism: list[NondeterminismRule] = Field(default_factory=list)
    expected_events: list[dict[str, Any]] = Field(default_factory=list)
    expected_event_count: int = 0
    tags: list[str] = Field(default_factory=list)


class ReplayDifference(BaseModel):
    """A difference between expected and actual events."""

    event_index: int
    field: str
    expected: Any
    actual: Any
    severity: str = "error"  # error, warning, info


class ReplayResult(BaseModel):
    """Result of a replay comparison."""

    success: bool
    manifest_name: str
    trace_id: UUID
    expected_count: int
    actual_count: int
    differences: list[ReplayDifference] = Field(default_factory=list)
    matched_events: int = 0
    skipped_fields: list[str] = Field(default_factory=list)
    duration_ms: int = 0


class TraceReplayer:
    """Replays and compares traces against golden expectations."""

    # Default nondeterminism rules
    DEFAULT_RULES = [
        NondeterminismRule(field="time", rule=NondeterminismRuleType.IGNORE),
        NondeterminismRule(field="*.time", rule=NondeterminismRuleType.IGNORE),
        NondeterminismRule(field="trace.span_id", rule=NondeterminismRuleType.NORMALIZE),
        NondeterminismRule(field="*.span_id", rule=NondeterminismRuleType.NORMALIZE),
        NondeterminismRule(field="execution_id", rule=NondeterminismRuleType.NORMALIZE),
        NondeterminismRule(field="*.execution_id", rule=NondeterminismRuleType.NORMALIZE),
    ]

    def __init__(self) -> None:
        """Initialize the replayer."""
        self._rules: list[NondeterminismRule] = list(self.DEFAULT_RULES)

    def add_rule(self, rule: NondeterminismRule) -> None:
        """Add a nondeterminism rule.

        Args:
            rule: The rule to add
        """
        self._rules.append(rule)

    def set_rules(self, rules: list[NondeterminismRule]) -> None:
        """Set all nondeterminism rules.

        Args:
            rules: The rules to use
        """
        self._rules = list(self.DEFAULT_RULES) + rules

    def compare(
        self,
        expected: list[dict[str, Any]],
        actual: list[TraceEvent],
        manifest_name: str = "unnamed",
        trace_id: UUID | None = None,
    ) -> ReplayResult:
        """Compare expected events against actual events.

        Args:
            expected: Expected events (from golden trace)
            actual: Actual events
            manifest_name: Name for the result
            trace_id: Trace ID

        Returns:
            ReplayResult with comparison details
        """
        start_time = datetime.utcnow()
        differences: list[ReplayDifference] = []
        matched = 0
        skipped_fields: set[str] = set()

        # Convert actual events to dicts
        actual_dicts = [e.model_dump() for e in actual]

        # Normalize both sets
        expected_normalized = [self._normalize(e) for e in expected]
        actual_normalized = [self._normalize(e) for e in actual_dicts]

        # Track which fields were skipped
        for rule in self._rules:
            if rule.rule == NondeterminismRuleType.IGNORE:
                skipped_fields.add(rule.field)

        # Compare event counts
        if len(expected) != len(actual):
            differences.append(
                ReplayDifference(
                    event_index=-1,
                    field="event_count",
                    expected=len(expected),
                    actual=len(actual),
                    severity="error",
                )
            )

        # Compare each event
        min_len = min(len(expected_normalized), len(actual_normalized))
        for i in range(min_len):
            event_diffs = self._compare_events(
                expected_normalized[i], actual_normalized[i], i
            )
            if event_diffs:
                differences.extend(event_diffs)
            else:
                matched += 1

        # Add differences for missing/extra events
        if len(expected) > len(actual):
            for i in range(len(actual), len(expected)):
                differences.append(
                    ReplayDifference(
                        event_index=i,
                        field="event",
                        expected=expected[i].get("event_type", "unknown"),
                        actual=None,
                        severity="error",
                    )
                )
        elif len(actual) > len(expected):
            for i in range(len(expected), len(actual)):
                differences.append(
                    ReplayDifference(
                        event_index=i,
                        field="event",
                        expected=None,
                        actual=actual_dicts[i].get("event_type", "unknown"),
                        severity="warning",
                    )
                )

        end_time = datetime.utcnow()
        duration_ms = int((end_time - start_time).total_seconds() * 1000)

        return ReplayResult(
            success=len(differences) == 0,
            manifest_name=manifest_name,
            trace_id=trace_id or UUID("00000000-0000-0000-0000-000000000000"),
            expected_count=len(expected),
            actual_count=len(actual),
            differences=differences,
            matched_events=matched,
            skipped_fields=list(skipped_fields),
            duration_ms=duration_ms,
        )

    def _normalize(self, event: dict[str, Any]) -> dict[str, Any]:
        """Normalize an event according to rules.

        Args:
            event: The event to normalize

        Returns:
            Normalized event
        """
        result = json.loads(json.dumps(event))  # Deep copy

        for rule in self._rules:
            self._apply_rule(result, rule, "")

        return result

    def _apply_rule(
        self, obj: Any, rule: NondeterminismRule, path: str
    ) -> None:
        """Apply a rule to an object.

        Args:
            obj: The object to modify
            rule: The rule to apply
            path: Current path in the object
        """
        if isinstance(obj, dict):
            for key, value in list(obj.items()):
                current_path = f"{path}.{key}" if path else key

                # Check if this field matches the rule
                if self._path_matches(current_path, rule.field):
                    if rule.rule == NondeterminismRuleType.IGNORE:
                        del obj[key]
                    elif rule.rule == NondeterminismRuleType.NORMALIZE:
                        obj[key] = f"<normalized:{key}>"
                    elif rule.rule == NondeterminismRuleType.MASK:
                        obj[key] = rule.value or "***"
                else:
                    # Recurse
                    self._apply_rule(value, rule, current_path)

        elif isinstance(obj, list):
            for i, item in enumerate(obj):
                self._apply_rule(item, rule, f"{path}[{i}]")

    def _path_matches(self, path: str, pattern: str) -> bool:
        """Check if a path matches a pattern.

        Args:
            path: The actual path
            pattern: The pattern (supports * wildcard)

        Returns:
            True if matches
        """
        # Convert pattern to regex
        regex = pattern.replace(".", r"\.").replace("*", r"[^.]+")
        regex = f"^{regex}$"

        # Also match just the field name
        field_name = path.split(".")[-1]
        simple_match = pattern == field_name or pattern == f"*.{field_name}"

        return bool(re.match(regex, path)) or simple_match

    def _compare_events(
        self,
        expected: dict[str, Any],
        actual: dict[str, Any],
        index: int,
        path: str = "",
    ) -> list[ReplayDifference]:
        """Compare two events recursively.

        Args:
            expected: Expected event
            actual: Actual event
            index: Event index
            path: Current path

        Returns:
            List of differences
        """
        differences: list[ReplayDifference] = []

        # Get all keys from both
        all_keys = set(expected.keys()) | set(actual.keys())

        for key in all_keys:
            current_path = f"{path}.{key}" if path else key
            exp_val = expected.get(key)
            act_val = actual.get(key)

            if key not in expected:
                differences.append(
                    ReplayDifference(
                        event_index=index,
                        field=current_path,
                        expected=None,
                        actual=act_val,
                        severity="warning",
                    )
                )
            elif key not in actual:
                differences.append(
                    ReplayDifference(
                        event_index=index,
                        field=current_path,
                        expected=exp_val,
                        actual=None,
                        severity="error",
                    )
                )
            elif isinstance(exp_val, dict) and isinstance(act_val, dict):
                differences.extend(
                    self._compare_events(exp_val, act_val, index, current_path)
                )
            elif isinstance(exp_val, list) and isinstance(act_val, list):
                if len(exp_val) != len(act_val):
                    differences.append(
                        ReplayDifference(
                            event_index=index,
                            field=f"{current_path}.length",
                            expected=len(exp_val),
                            actual=len(act_val),
                            severity="error",
                        )
                    )
                else:
                    for i, (e, a) in enumerate(zip(exp_val, act_val)):
                        if isinstance(e, dict) and isinstance(a, dict):
                            differences.extend(
                                self._compare_events(e, a, index, f"{current_path}[{i}]")
                            )
                        elif e != a:
                            differences.append(
                                ReplayDifference(
                                    event_index=index,
                                    field=f"{current_path}[{i}]",
                                    expected=e,
                                    actual=a,
                                    severity="error",
                                )
                            )
            elif exp_val != act_val:
                differences.append(
                    ReplayDifference(
                        event_index=index,
                        field=current_path,
                        expected=exp_val,
                        actual=act_val,
                        severity="error",
                    )
                )

        return differences

    def create_manifest(
        self,
        events: list[TraceEvent],
        name: str,
        description: str = "",
        tags: list[str] | None = None,
    ) -> ReplayManifest:
        """Create a replay manifest from events.

        Args:
            events: The events to record
            name: Manifest name
            description: Description
            tags: Optional tags

        Returns:
            The replay manifest
        """
        if not events:
            raise ValueError("Cannot create manifest from empty events")

        trace_id = events[0].trace.trace_id

        # Convert events to dicts
        event_dicts = [e.model_dump(mode="json") for e in events]

        return ReplayManifest(
            trace_id=trace_id,
            name=name,
            description=description,
            expected_events=event_dicts,
            expected_event_count=len(events),
            tags=tags or [],
            nondeterminism=list(self.DEFAULT_RULES),
        )

    def save_manifest(self, manifest: ReplayManifest, path: Path) -> None:
        """Save a manifest to a file.

        Args:
            manifest: The manifest to save
            path: Output path
        """
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w") as f:
            json.dump(manifest.model_dump(mode="json"), f, indent=2, default=str)
            f.write("\n")

    def load_manifest(self, path: Path) -> ReplayManifest:
        """Load a manifest from a file.

        Args:
            path: Path to manifest

        Returns:
            The loaded manifest
        """
        with open(path) as f:
            data = json.load(f)
        return ReplayManifest(**data)


# Global replayer instance
_replayer: TraceReplayer | None = None


def get_replayer() -> TraceReplayer:
    """Get the global trace replayer instance."""
    global _replayer
    if _replayer is None:
        _replayer = TraceReplayer()
    return _replayer
