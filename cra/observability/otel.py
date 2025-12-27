"""OpenTelemetry export for CRA TRACE events.

Exports CRA TRACE events as OpenTelemetry spans.
Note: TRACE is canonical; OTel is derivative.
"""

import os
from datetime import datetime
from typing import Any
from uuid import UUID

from cra.core.trace import TraceEvent, Severity


class OTelExporter:
    """Export CRA TRACE events to OpenTelemetry.

    This exporter converts CRA TRACE events to OpenTelemetry spans
    for integration with existing observability infrastructure.

    Important: TRACE remains the authoritative record.
    OTel export is for convenience and integration only.
    """

    def __init__(
        self,
        service_name: str = "cra-runtime",
        endpoint: str | None = None,
        headers: dict[str, str] | None = None,
    ):
        self.service_name = service_name
        self.endpoint = endpoint or os.getenv(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "http://localhost:4317",
        )
        self.headers = headers or {}

        self._tracer: Any = None
        self._provider: Any = None

    def setup(self) -> None:
        """Set up OpenTelemetry tracing."""
        try:
            from opentelemetry import trace
            from opentelemetry.sdk.trace import TracerProvider
            from opentelemetry.sdk.trace.export import BatchSpanProcessor
            from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import (
                OTLPSpanExporter,
            )
            from opentelemetry.sdk.resources import Resource, SERVICE_NAME
        except ImportError:
            raise RuntimeError(
                "OpenTelemetry packages required. Install with: "
                "pip install opentelemetry-api opentelemetry-sdk "
                "opentelemetry-exporter-otlp-proto-grpc"
            )

        # Create resource
        resource = Resource.create({SERVICE_NAME: self.service_name})

        # Create provider
        self._provider = TracerProvider(resource=resource)

        # Create exporter
        exporter = OTLPSpanExporter(
            endpoint=self.endpoint,
            headers=self.headers,
        )

        # Add processor
        self._provider.add_span_processor(BatchSpanProcessor(exporter))

        # Set as global provider
        trace.set_tracer_provider(self._provider)

        # Get tracer
        self._tracer = trace.get_tracer("cra")

    def export_event(self, event: TraceEvent) -> None:
        """Export a single TRACE event as an OTel span.

        Args:
            event: The TRACE event to export
        """
        if not self._tracer:
            self.setup()

        from opentelemetry import trace
        from opentelemetry.trace import Status, StatusCode, SpanKind

        # Map TRACE severity to OTel status
        status_map = {
            Severity.DEBUG: StatusCode.OK,
            Severity.INFO: StatusCode.OK,
            Severity.WARN: StatusCode.OK,
            Severity.ERROR: StatusCode.ERROR,
        }

        # Create span context from TRACE context
        span_context = trace.SpanContext(
            trace_id=self._uuid_to_trace_id(event.trace.trace_id),
            span_id=self._uuid_to_span_id(event.trace.span_id),
            is_remote=False,
            trace_flags=trace.TraceFlags(0x01),  # Sampled
        )

        # Determine parent context
        parent_context = None
        if event.trace.parent_span_id:
            parent_span_context = trace.SpanContext(
                trace_id=self._uuid_to_trace_id(event.trace.trace_id),
                span_id=self._uuid_to_span_id(event.trace.parent_span_id),
                is_remote=False,
                trace_flags=trace.TraceFlags(0x01),
            )
            parent_context = trace.set_span_in_context(
                trace.NonRecordingSpan(parent_span_context)
            )

        # Create the span
        with self._tracer.start_as_current_span(
            name=event.event_type,
            context=parent_context,
            kind=SpanKind.INTERNAL,
            start_time=self._datetime_to_ns(event.time),
        ) as span:
            # Set attributes
            span.set_attribute("cra.trace_version", event.trace_version)
            span.set_attribute("cra.event_type", event.event_type)
            span.set_attribute("cra.session_id", str(event.session_id))
            span.set_attribute("cra.actor.type", event.actor.type.value)
            span.set_attribute("cra.actor.id", event.actor.id)
            span.set_attribute("cra.severity", event.severity.value)

            if event.atlas:
                span.set_attribute("cra.atlas.id", event.atlas.id)
                if event.atlas.version:
                    span.set_attribute("cra.atlas.version", event.atlas.version)

            # Add payload as attributes
            if event.payload:
                for key, value in self._flatten_dict(event.payload, "cra.payload").items():
                    if isinstance(value, (str, int, float, bool)):
                        span.set_attribute(key, value)

            # Set status
            span.set_status(Status(status_map[event.severity]))

    def export_events(self, events: list[TraceEvent]) -> None:
        """Export multiple TRACE events.

        Args:
            events: List of events to export
        """
        for event in events:
            self.export_event(event)

    def shutdown(self) -> None:
        """Shut down the exporter."""
        if self._provider:
            self._provider.shutdown()

    def _uuid_to_trace_id(self, uuid_val: UUID) -> int:
        """Convert UUID to 128-bit trace ID."""
        return uuid_val.int & ((1 << 128) - 1)

    def _uuid_to_span_id(self, uuid_val: UUID) -> int:
        """Convert UUID to 64-bit span ID."""
        return uuid_val.int & ((1 << 64) - 1)

    def _datetime_to_ns(self, dt: datetime) -> int:
        """Convert datetime to nanoseconds since epoch."""
        return int(dt.timestamp() * 1_000_000_000)

    def _flatten_dict(
        self,
        d: dict[str, Any],
        prefix: str = "",
    ) -> dict[str, Any]:
        """Flatten a nested dictionary."""
        items: dict[str, Any] = {}
        for key, value in d.items():
            new_key = f"{prefix}.{key}" if prefix else key
            if isinstance(value, dict):
                items.update(self._flatten_dict(value, new_key))
            else:
                items[new_key] = value
        return items


def setup_otel_export(
    service_name: str = "cra-runtime",
    endpoint: str | None = None,
) -> OTelExporter:
    """Set up OpenTelemetry export.

    Args:
        service_name: Service name for tracing
        endpoint: OTLP endpoint URL

    Returns:
        Configured OTel exporter
    """
    exporter = OTelExporter(service_name, endpoint)
    exporter.setup()
    return exporter
