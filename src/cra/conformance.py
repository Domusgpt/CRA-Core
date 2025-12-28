from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from .runtime import CRARuntime
from .trace import TraceStore


class ConformanceRunner:
    """Lightweight harness to validate adapter scope fixtures and TRACE replays."""

    def __init__(self, atlas_path: Path | str = Path("atlas/reference"), traces_dir: Path | str = Path("traces")) -> None:
        self.runtime = CRARuntime(atlas_path)
        self.trace_store = TraceStore(traces_dir)

    def check_adapter_scopes(self, fixture_path: Path | str) -> Tuple[bool, List[str]]:
        fixture = json.loads(Path(fixture_path).read_text())
        declared_actions = {a.action_id: a for a in self.runtime.atlas_loader.allowed_actions()}
        errors: List[str] = []

        for expected in fixture.get("actions", []):
            action_id = expected.get("action_id")
            action = declared_actions.get(action_id)
            if not action:
                errors.append(f"Missing action in atlas: {action_id}")
                continue

            expected_scopes = sorted(expected.get("required_scopes", []))
            action_scopes = sorted(action.required_scopes)
            if expected_scopes != action_scopes:
                errors.append(
                    f"Action {action_id} scope mismatch: expected {expected_scopes}, got {action_scopes}"
                )

            expected_risk = expected.get("risk")
            if expected_risk and action.risk != expected_risk:
                errors.append(f"Action {action_id} risk mismatch: expected {expected_risk}, got {action.risk}")

            expected_approval = expected.get("approval_policy")
            if expected_approval and action.approval_policy != expected_approval:
                errors.append(
                    f"Action {action_id} approval mismatch: expected {expected_approval}, got {action.approval_policy}"
                )

            expected_rate = expected.get("rate_limit")
            if expected_rate:
                actual_rate = action.rate_limit or {}
                actual_limit = actual_rate.get("limit") or actual_rate.get("value")
                if expected_rate.get("limit") != actual_limit:
                    errors.append(
                        f"Action {action_id} rate limit mismatch: expected {expected_rate.get('limit')}, got {actual_limit}"
                    )
                if expected_rate.get("window_seconds") and expected_rate.get("window_seconds") != actual_rate.get(
                    "window_seconds"
                ):
                    errors.append(
                        f"Action {action_id} rate window mismatch: expected {expected_rate.get('window_seconds')}, "
                        f"got {actual_rate.get('window_seconds')}"
                    )

        return not errors, errors

    def check_trace_replay(self, trace_file: Path | str, fail_fast: bool = False) -> Tuple[bool, List[str]]:
        count, errors = self.trace_store.replay(trace_file, fail_fast=fail_fast)
        if not errors and count == 0:
            errors.append("No events found in trace file")
        return not errors, errors

    def run(
        self,
        fixture_path: Path | str,
        trace_file: Optional[Path | str] = None,
        fail_fast: bool = False,
    ) -> Dict[str, Dict[str, List[str] | bool]]:
        results: Dict[str, Dict[str, List[str] | bool]] = {}
        adapter_passed, adapter_errors = self.check_adapter_scopes(fixture_path)
        results["adapter_scopes"] = {"passed": adapter_passed, "errors": adapter_errors}

        if trace_file:
            trace_passed, trace_errors = self.check_trace_replay(trace_file, fail_fast=fail_fast)
            results["trace_replay"] = {"passed": trace_passed, "errors": trace_errors}

        results["ok"] = all(section.get("passed", True) for section in results.values())
        return results


__all__ = ["ConformanceRunner"]
