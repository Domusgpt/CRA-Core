"""Observability exports for CRA.

Provides integration with external observability systems.
TRACE remains canonical; exports are derivative.
"""

from cra.observability.otel import OTelExporter, setup_otel_export
from cra.observability.siem import SIEMExporter, SIEMFormat

__all__ = [
    "OTelExporter",
    "setup_otel_export",
    "SIEMExporter",
    "SIEMFormat",
]
