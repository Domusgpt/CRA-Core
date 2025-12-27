"""FastAPI dependencies for runtime API."""

from cra.runtime.services.resolver import Resolver, get_resolver
from cra.runtime.services.session_manager import SessionManager, get_session_manager
from cra.runtime.services.tracer import Tracer, get_tracer


def get_tracer_dep() -> Tracer:
    """Dependency for getting the tracer service."""
    return get_tracer()


def get_session_manager_dep() -> SessionManager:
    """Dependency for getting the session manager service."""
    tracer = get_tracer()
    return get_session_manager(tracer)


def get_resolver_dep() -> Resolver:
    """Dependency for getting the resolver service."""
    tracer = get_tracer()
    session_manager = get_session_manager(tracer)
    return get_resolver(tracer, session_manager)
