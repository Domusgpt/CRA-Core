from __future__ import annotations

from typing import Dict

from jsonschema import Draft202012Validator, ValidationError


CARP_REQUEST_SCHEMA: Dict = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["carp_version", "type", "id", "time", "session", "atlas", "payload", "trace"],
    "properties": {
        "carp_version": {"type": "string"},
        "type": {"const": "carp.request"},
        "id": {"type": "string"},
        "time": {"type": "string"},
        "session": {"type": "object"},
        "atlas": {"type": "object"},
        "payload": {
            "type": "object",
            "required": ["operation", "task"],
            "properties": {
                "operation": {"const": "resolve"},
                "task": {
                    "type": "object",
                    "required": ["goal", "target_platforms", "risk_tier", "constraints", "inputs"],
                    "properties": {
                        "goal": {"type": "string"},
                        "target_platforms": {"type": "array", "items": {"type": "string"}},
                        "risk_tier": {"type": "string"},
                        "constraints": {"type": "array"},
                        "inputs": {"type": "array"},
                    },
                },
                "environment": {"type": "object"},
                "preferences": {"type": "object"},
            },
        },
        "trace": {"type": "object"},
    },
}


CARP_RESPONSE_SCHEMA: Dict = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["carp_version", "type", "id", "time", "session", "atlas", "payload", "trace"],
    "properties": {
        "carp_version": {"type": "string"},
        "type": {"const": "carp.response"},
        "id": {"type": "string"},
        "time": {"type": "string"},
        "session": {"type": "object"},
        "atlas": {"type": "object"},
        "payload": {
            "type": "object",
            "required": ["operation", "resolution"],
            "properties": {
                "operation": {"const": "resolve"},
                "resolution": {"type": "object"},
            },
        },
        "trace": {"type": "object"},
    },
}


TRACE_EVENT_SCHEMA: Dict = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": [
        "trace_version",
        "event_type",
        "time",
        "trace",
        "session_id",
        "atlas",
        "actor",
        "severity",
        "payload",
        "artifacts",
    ],
    "properties": {
        "trace_version": {"type": "string"},
        "event_type": {"type": "string"},
        "time": {"type": "string"},
        "trace": {"type": "object"},
        "session_id": {"type": "string"},
        "atlas": {"type": "object"},
        "actor": {"type": "object"},
        "severity": {"type": "string"},
        "payload": {"type": "object"},
        "artifacts": {"type": "array"},
    },
}


ATLAS_SCHEMA: Dict = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["atlas_version", "id", "name", "version", "publisher", "capabilities", "platform_adapters"],
    "properties": {
        "atlas_version": {"type": "string"},
        "id": {"type": "string"},
        "name": {"type": "string"},
        "version": {"type": "string"},
        "publisher": {"type": "object"},
        "capabilities": {"type": "array"},
        "platform_adapters": {"type": "array", "items": {"type": "string"}},
        "licensing": {"type": "object"},
    },
}


class SchemaValidator:
    """Lightweight JSON Schema validation helpers for CARP/TRACE/Atlas documents."""

    def __init__(self) -> None:
        self.validators = {
            "carp.request": Draft202012Validator(CARP_REQUEST_SCHEMA),
            "carp.response": Draft202012Validator(CARP_RESPONSE_SCHEMA),
            "trace.event": Draft202012Validator(TRACE_EVENT_SCHEMA),
            "atlas": Draft202012Validator(ATLAS_SCHEMA),
        }

    def validate(self, name: str, payload: Dict) -> None:
        if name not in self.validators:
            raise ValueError(f"Unknown schema name: {name}")
        self.validators[name].validate(payload)


__all__ = [
    "SchemaValidator",
    "ValidationError",
]
