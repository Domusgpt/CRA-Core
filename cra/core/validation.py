"""JSON Schema validation for CARP and TRACE.

Provides strict validation for all protocol messages.
Exports Pydantic models as JSON Schema for external tooling.
"""

import json
from pathlib import Path
from typing import Any, Type

from pydantic import BaseModel, ValidationError

from cra.core.carp import (
    CARPEnvelope,
    CARPRequest,
    CARPResponse,
)
from cra.core.trace import TraceEvent


class SchemaValidationError(Exception):
    """Raised when schema validation fails."""

    def __init__(self, message: str, errors: list[dict[str, Any]]) -> None:
        super().__init__(message)
        self.errors = errors


class SchemaValidator:
    """Validates CARP and TRACE messages against their schemas.

    All validation uses Pydantic's strict mode to ensure
    type correctness and protocol compliance.
    """

    def __init__(self) -> None:
        """Initialize the validator."""
        self._schemas: dict[str, dict[str, Any]] = {}

    def validate_carp_request(self, data: dict[str, Any]) -> CARPRequest:
        """Validate a CARP request.

        Args:
            data: Raw request data

        Returns:
            Validated CARPRequest

        Raises:
            SchemaValidationError: If validation fails
        """
        return self._validate(CARPRequest, data, "CARP request")

    def validate_carp_response(self, data: dict[str, Any]) -> CARPResponse:
        """Validate a CARP response.

        Args:
            data: Raw response data

        Returns:
            Validated CARPResponse

        Raises:
            SchemaValidationError: If validation fails
        """
        return self._validate(CARPResponse, data, "CARP response")

    def validate_trace_event(self, data: dict[str, Any]) -> TraceEvent:
        """Validate a TRACE event.

        Args:
            data: Raw event data

        Returns:
            Validated TraceEvent

        Raises:
            SchemaValidationError: If validation fails
        """
        return self._validate(TraceEvent, data, "TRACE event")

    def _validate(
        self, model: Type[BaseModel], data: dict[str, Any], context: str
    ) -> BaseModel:
        """Validate data against a Pydantic model.

        Args:
            model: The Pydantic model class
            data: Data to validate
            context: Context for error messages

        Returns:
            Validated model instance

        Raises:
            SchemaValidationError: If validation fails
        """
        try:
            return model.model_validate(data, strict=True)
        except ValidationError as e:
            errors = [
                {
                    "loc": list(err["loc"]),
                    "msg": err["msg"],
                    "type": err["type"],
                }
                for err in e.errors()
            ]
            raise SchemaValidationError(
                f"Invalid {context}: {len(errors)} validation error(s)",
                errors=errors,
            )

    def get_json_schema(self, model: Type[BaseModel]) -> dict[str, Any]:
        """Get JSON Schema for a Pydantic model.

        Args:
            model: The Pydantic model class

        Returns:
            JSON Schema dictionary
        """
        model_name = model.__name__
        if model_name not in self._schemas:
            self._schemas[model_name] = model.model_json_schema()
        return self._schemas[model_name]

    def export_schemas(self, output_dir: Path) -> None:
        """Export all schemas to JSON files.

        Args:
            output_dir: Directory to write schema files
        """
        output_dir.mkdir(parents=True, exist_ok=True)

        schemas = [
            (CARPRequest, "carp_request_v1.json"),
            (CARPResponse, "carp_response_v1.json"),
            (CARPEnvelope, "carp_envelope_v1.json"),
            (TraceEvent, "trace_event_v1.json"),
        ]

        for model, filename in schemas:
            schema = self.get_json_schema(model)
            schema_path = output_dir / filename
            with open(schema_path, "w") as f:
                json.dump(schema, f, indent=2)
                f.write("\n")


# Global validator instance
_validator: SchemaValidator | None = None


def get_validator() -> SchemaValidator:
    """Get the global schema validator instance."""
    global _validator
    if _validator is None:
        _validator = SchemaValidator()
    return _validator
