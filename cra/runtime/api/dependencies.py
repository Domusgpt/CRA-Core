"""FastAPI dependencies for runtime API."""

from cra.core.policy import PolicyEngine, get_policy_engine
from cra.core.validation import SchemaValidator, get_validator
from cra.runtime.services.executor import ActionExecutor, get_executor
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


def get_policy_engine_dep() -> PolicyEngine:
    """Dependency for getting the policy engine."""
    return get_policy_engine()


def get_validator_dep() -> SchemaValidator:
    """Dependency for getting the schema validator."""
    return get_validator()


def get_resolver_dep() -> Resolver:
    """Dependency for getting the resolver service."""
    tracer = get_tracer()
    session_manager = get_session_manager(tracer)
    policy_engine = get_policy_engine()
    return get_resolver(tracer, session_manager, policy_engine)


def get_executor_dep() -> ActionExecutor:
    """Dependency for getting the action executor."""
    tracer = get_tracer()
    session_manager = get_session_manager(tracer)
    return get_executor(tracer, session_manager)
