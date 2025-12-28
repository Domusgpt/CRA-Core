"""SIEM export for CRA TRACE events.

Exports CRA TRACE events to SIEM systems in various formats.
"""

import json
import os
from datetime import datetime
from enum import Enum
from typing import Any, Callable
from uuid import UUID

from cra.core.trace import TraceEvent, Severity


class SIEMFormat(str, Enum):
    """Supported SIEM export formats."""

    CEF = "cef"  # Common Event Format (ArcSight)
    LEEF = "leef"  # Log Event Extended Format (IBM QRadar)
    JSON = "json"  # Generic JSON
    SYSLOG = "syslog"  # RFC 5424 Syslog


class SIEMExporter:
    """Export CRA TRACE events to SIEM systems.

    Supports multiple output formats for different SIEM platforms.
    """

    def __init__(
        self,
        format: SIEMFormat = SIEMFormat.JSON,
        output: Callable[[str], None] | None = None,
        vendor: str = "CRA",
        product: str = "Runtime",
        version: str = "1.0",
    ):
        self.format = format
        self.output = output or self._default_output
        self.vendor = vendor
        self.product = product
        self.version = version

        self._formatters: dict[SIEMFormat, Callable[[TraceEvent], str]] = {
            SIEMFormat.CEF: self._format_cef,
            SIEMFormat.LEEF: self._format_leef,
            SIEMFormat.JSON: self._format_json,
            SIEMFormat.SYSLOG: self._format_syslog,
        }

    def export_event(self, event: TraceEvent) -> str:
        """Export a TRACE event in the configured format.

        Args:
            event: The event to export

        Returns:
            Formatted event string
        """
        formatter = self._formatters[self.format]
        formatted = formatter(event)
        self.output(formatted)
        return formatted

    def export_events(self, events: list[TraceEvent]) -> list[str]:
        """Export multiple events.

        Args:
            events: Events to export

        Returns:
            List of formatted event strings
        """
        return [self.export_event(event) for event in events]

    def _format_cef(self, event: TraceEvent) -> str:
        """Format event as CEF (Common Event Format).

        CEF format: CEF:Version|Device Vendor|Device Product|Device Version|
                    Signature ID|Name|Severity|Extension
        """
        # Map severity to CEF severity (0-10)
        severity_map = {
            Severity.DEBUG: 1,
            Severity.INFO: 3,
            Severity.WARN: 6,
            Severity.ERROR: 9,
        }

        # Build extension
        extension_parts = [
            f"rt={self._format_time_cef(event.time)}",
            f"src={event.actor.id}",
            f"suser={event.actor.type.value}",
            f"cs1={str(event.trace.trace_id)}",
            f"cs1Label=TraceID",
            f"cs2={str(event.session_id)}",
            f"cs2Label=SessionID",
        ]

        if event.atlas:
            extension_parts.append(f"cs3={event.atlas.id}")
            extension_parts.append("cs3Label=AtlasID")

        if event.payload:
            # Encode payload as custom string
            payload_str = json.dumps(event.payload)
            # Escape special CEF characters
            payload_str = payload_str.replace("\\", "\\\\").replace("=", "\\=")
            extension_parts.append(f"msg={payload_str}")

        extension = " ".join(extension_parts)

        return (
            f"CEF:0|{self.vendor}|{self.product}|{self.version}|"
            f"{event.event_type}|{event.event_type}|"
            f"{severity_map[event.severity]}|{extension}"
        )

    def _format_leef(self, event: TraceEvent) -> str:
        """Format event as LEEF (Log Event Extended Format).

        LEEF format: LEEF:Version|Vendor|Product|Version|EventID|
                     Key1=Value1<tab>Key2=Value2...
        """
        # Build attributes
        attrs = [
            f"devTime={self._format_time_leef(event.time)}",
            f"src={event.actor.id}",
            f"usrName={event.actor.type.value}",
            f"traceId={event.trace.trace_id}",
            f"sessionId={event.session_id}",
            f"severity={event.severity.value}",
        ]

        if event.atlas:
            attrs.append(f"atlasId={event.atlas.id}")

        if event.payload:
            attrs.append(f"payload={json.dumps(event.payload)}")

        return (
            f"LEEF:2.0|{self.vendor}|{self.product}|{self.version}|"
            f"{event.event_type}|" + "\t".join(attrs)
        )

    def _format_json(self, event: TraceEvent) -> str:
        """Format event as JSON for generic SIEM ingestion."""
        return event.model_dump_json()

    def _format_syslog(self, event: TraceEvent) -> str:
        """Format event as RFC 5424 Syslog.

        Format: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID
                STRUCTURED-DATA MSG
        """
        # Map severity to syslog priority
        # Facility 16 (local0), severity based on event
        severity_map = {
            Severity.DEBUG: 7,  # Debug
            Severity.INFO: 6,  # Informational
            Severity.WARN: 4,  # Warning
            Severity.ERROR: 3,  # Error
        }

        pri = (16 * 8) + severity_map[event.severity]  # Facility 16 * 8 + severity

        # Format timestamp as RFC 5424
        timestamp = event.time.strftime("%Y-%m-%dT%H:%M:%S.%fZ")

        # Structured data
        sd_params = [
            f'traceId="{event.trace.trace_id}"',
            f'spanId="{event.trace.span_id}"',
            f'sessionId="{event.session_id}"',
            f'actorType="{event.actor.type.value}"',
            f'actorId="{event.actor.id}"',
        ]
        if event.atlas:
            sd_params.append(f'atlasId="{event.atlas.id}"')

        structured_data = f"[cra@12345 {' '.join(sd_params)}]"

        # Message
        msg = event.event_type
        if event.payload:
            msg += f" {json.dumps(event.payload)}"

        hostname = os.getenv("HOSTNAME", "cra-runtime")

        return (
            f"<{pri}>1 {timestamp} {hostname} cra - {event.event_type} "
            f"{structured_data} {msg}"
        )

    def _format_time_cef(self, dt: datetime) -> str:
        """Format time for CEF (milliseconds since epoch)."""
        return str(int(dt.timestamp() * 1000))

    def _format_time_leef(self, dt: datetime) -> str:
        """Format time for LEEF."""
        return dt.strftime("%b %d %Y %H:%M:%S")

    def _default_output(self, formatted: str) -> None:
        """Default output handler (print to stdout)."""
        print(formatted)


def create_siem_exporter(
    format: str | SIEMFormat = SIEMFormat.JSON,
    output: Callable[[str], None] | None = None,
) -> SIEMExporter:
    """Create a SIEM exporter.

    Args:
        format: Export format
        output: Output handler function

    Returns:
        Configured SIEM exporter
    """
    if isinstance(format, str):
        format = SIEMFormat(format)
    return SIEMExporter(format=format, output=output)
