"""Pytest configuration and fixtures."""

import pytest
from fastapi.testclient import TestClient

from cra.runtime.server import create_app


@pytest.fixture
def app():
    """Create a fresh app instance for testing."""
    return create_app()


@pytest.fixture
def client(app):
    """Create a test client."""
    return TestClient(app)


@pytest.fixture
def session_data(client):
    """Create a session and return the session data."""
    response = client.post(
        "/v1/sessions",
        json={
            "principal": {"type": "user", "id": "test-user"},
            "scopes": ["carp.resolve"],
            "ttl_seconds": 3600,
        },
    )
    assert response.status_code == 201
    return response.json()
