"""CRA settings and configuration.

Supports environment-specific configuration with validation.
"""

import os
from enum import Enum
from functools import lru_cache
from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Environment(str, Enum):
    """Deployment environments."""

    DEVELOPMENT = "development"
    STAGING = "staging"
    PRODUCTION = "production"


class RuntimeSettings(BaseModel):
    """Runtime server settings."""

    host: str = "127.0.0.1"
    port: int = 8420
    workers: int = 1
    reload: bool = False
    log_level: str = "info"
    cors_origins: list[str] = Field(default_factory=lambda: ["*"])
    request_timeout_seconds: int = 30
    max_request_size_mb: int = 10


class AuthSettings(BaseModel):
    """Authentication settings."""

    enabled: bool = True
    jwt_secret: str = Field(default="change-me-in-production")
    jwt_algorithm: str = "HS256"
    jwt_access_token_expire_minutes: int = 60
    jwt_refresh_token_expire_days: int = 7
    api_key_enabled: bool = True
    require_auth_for_health: bool = False
    exempt_paths: list[str] = Field(
        default_factory=lambda: ["/v1/health", "/docs", "/redoc", "/openapi.json"]
    )

    @field_validator("jwt_secret")
    @classmethod
    def validate_jwt_secret(cls, v: str, info: Any) -> str:
        if v == "change-me-in-production":
            env = os.getenv("CRA_ENV", "development")
            if env == "production":
                raise ValueError("JWT secret must be changed in production")
        return v


class StorageSettings(BaseModel):
    """Storage settings."""

    backend: str = "memory"  # memory, postgres
    postgres_url: str | None = None
    postgres_pool_size: int = 10
    trace_retention_days: int = 30
    session_cleanup_interval_seconds: int = 300


class ObservabilitySettings(BaseModel):
    """Observability settings."""

    otel_enabled: bool = False
    otel_endpoint: str = "http://localhost:4317"
    otel_service_name: str = "cra-runtime"
    siem_enabled: bool = False
    siem_format: str = "json"  # json, cef, leef, syslog
    metrics_enabled: bool = True
    metrics_port: int = 9090


class AtlasSettings(BaseModel):
    """Atlas registry settings."""

    auto_load_paths: list[str] = Field(default_factory=list)
    cache_enabled: bool = True
    cache_ttl_seconds: int = 3600
    validation_strict: bool = True


class Settings(BaseSettings):
    """Main CRA settings.

    Loaded from environment variables with CRA_ prefix.
    """

    model_config = SettingsConfigDict(
        env_prefix="CRA_",
        env_nested_delimiter="__",
        case_sensitive=False,
    )

    # Environment
    env: Environment = Environment.DEVELOPMENT
    debug: bool = False
    version: str = "0.1.0"

    # Nested settings
    runtime: RuntimeSettings = Field(default_factory=RuntimeSettings)
    auth: AuthSettings = Field(default_factory=AuthSettings)
    storage: StorageSettings = Field(default_factory=StorageSettings)
    observability: ObservabilitySettings = Field(default_factory=ObservabilitySettings)
    atlas: AtlasSettings = Field(default_factory=AtlasSettings)

    def is_production(self) -> bool:
        """Check if running in production."""
        return self.env == Environment.PRODUCTION

    def is_development(self) -> bool:
        """Check if running in development."""
        return self.env == Environment.DEVELOPMENT


def load_settings(
    env_file: str | Path | None = None,
    overrides: dict[str, Any] | None = None,
) -> Settings:
    """Load settings from environment.

    Args:
        env_file: Optional .env file path
        overrides: Optional setting overrides

    Returns:
        Loaded settings
    """
    if env_file:
        from dotenv import load_dotenv
        load_dotenv(env_file)

    settings = Settings()

    if overrides:
        # Apply overrides
        for key, value in overrides.items():
            if hasattr(settings, key):
                setattr(settings, key, value)

    return settings


@lru_cache
def get_settings() -> Settings:
    """Get cached settings singleton."""
    return load_settings()


# Environment-specific configuration presets

DEVELOPMENT_PRESET = {
    "env": Environment.DEVELOPMENT,
    "debug": True,
    "runtime": RuntimeSettings(
        reload=True,
        log_level="debug",
    ),
    "auth": AuthSettings(
        enabled=False,
    ),
    "storage": StorageSettings(
        backend="memory",
    ),
}

STAGING_PRESET = {
    "env": Environment.STAGING,
    "debug": False,
    "runtime": RuntimeSettings(
        workers=2,
        log_level="info",
    ),
    "auth": AuthSettings(
        enabled=True,
    ),
    "storage": StorageSettings(
        backend="postgres",
    ),
    "observability": ObservabilitySettings(
        otel_enabled=True,
    ),
}

PRODUCTION_PRESET = {
    "env": Environment.PRODUCTION,
    "debug": False,
    "runtime": RuntimeSettings(
        host="0.0.0.0",
        workers=4,
        log_level="warning",
        cors_origins=[],  # Must be configured explicitly
    ),
    "auth": AuthSettings(
        enabled=True,
        jwt_secret="MUST_BE_SET",  # Will fail validation
    ),
    "storage": StorageSettings(
        backend="postgres",
        postgres_pool_size=20,
    ),
    "observability": ObservabilitySettings(
        otel_enabled=True,
        siem_enabled=True,
        metrics_enabled=True,
    ),
}
