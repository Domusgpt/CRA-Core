"""Integration tests for the CRA Runtime API."""

from datetime import datetime
from uuid import uuid4

import pytest


class TestHealthEndpoint:
    """Tests for /v1/health endpoint."""

    def test_health_check(self, client):
        """Test health check returns healthy status."""
        response = client.get("/v1/health")
        assert response.status_code == 200

        data = response.json()
        assert data["status"] == "healthy"
        assert data["version"] == "0.1.0"
        assert data["carp_version"] == "1.0"
        assert data["trace_version"] == "1.0"
        assert "uptime_seconds" in data


class TestSessionsEndpoint:
    """Tests for /v1/sessions endpoints."""

    def test_create_session(self, client):
        """Test creating a new session."""
        response = client.post(
            "/v1/sessions",
            json={
                "principal": {"type": "user", "id": "test-user"},
                "scopes": ["carp.resolve"],
                "ttl_seconds": 3600,
            },
        )
        assert response.status_code == 201

        data = response.json()
        assert "session_id" in data
        assert "trace_id" in data
        assert "expires_at" in data

    def test_end_session(self, client, session_data):
        """Test ending a session."""
        session_id = session_data["session_id"]

        response = client.post(f"/v1/sessions/{session_id}/end")
        assert response.status_code == 200

        data = response.json()
        assert data["session_id"] == session_id
        assert "ended_at" in data
        assert "trace_summary" in data

    def test_end_nonexistent_session(self, client):
        """Test ending a session that doesn't exist."""
        fake_id = str(uuid4())
        response = client.post(f"/v1/sessions/{fake_id}/end")
        assert response.status_code == 404


class TestCARPEndpoint:
    """Tests for /v1/carp endpoints."""

    def test_resolve(self, client, session_data):
        """Test CARP resolution."""
        session_id = session_data["session_id"]
        trace_id = session_data["trace_id"]

        request = {
            "carp_version": "1.0",
            "type": "carp.request",
            "id": str(uuid4()),
            "time": datetime.utcnow().isoformat() + "Z",
            "session": {
                "session_id": session_id,
                "principal": {"type": "user", "id": "test-user"},
                "scopes": ["carp.resolve"],
            },
            "atlas": None,
            "payload": {
                "operation": "resolve",
                "task": {
                    "goal": "Test task",
                    "inputs": [],
                    "constraints": [],
                    "target_platforms": ["openai.tools"],
                    "risk_tier": "medium",
                },
                "environment": {
                    "project_root": None,
                    "os": None,
                    "cli_capabilities": ["bash"],
                    "network_policy": "open",
                },
                "preferences": {
                    "verbosity": "standard",
                    "format": ["json"],
                    "explainability": "standard",
                },
            },
            "trace": {
                "trace_id": trace_id,
                "span_id": str(uuid4()),
                "parent_span_id": None,
            },
        }

        response = client.post("/v1/carp/resolve", json=request)
        assert response.status_code == 200

        data = response.json()
        assert data["type"] == "carp.response"
        assert "payload" in data
        assert "resolution" in data["payload"]

        resolution = data["payload"]["resolution"]
        assert "resolution_id" in resolution
        assert "confidence" in resolution
        assert "context_blocks" in resolution
        assert "allowed_actions" in resolution
        assert "denylist" in resolution

    def test_resolve_with_invalid_session(self, client):
        """Test CARP resolution with invalid session."""
        fake_session_id = str(uuid4())
        trace_id = str(uuid4())

        request = {
            "carp_version": "1.0",
            "type": "carp.request",
            "id": str(uuid4()),
            "time": datetime.utcnow().isoformat() + "Z",
            "session": {
                "session_id": fake_session_id,
                "principal": {"type": "user", "id": "test-user"},
                "scopes": ["carp.resolve"],
            },
            "atlas": None,
            "payload": {
                "operation": "resolve",
                "task": {
                    "goal": "Test task",
                    "inputs": [],
                    "constraints": [],
                    "target_platforms": ["openai.tools"],
                    "risk_tier": "medium",
                },
                "environment": {},
                "preferences": {},
            },
            "trace": {
                "trace_id": trace_id,
                "span_id": str(uuid4()),
                "parent_span_id": None,
            },
        }

        response = client.post("/v1/carp/resolve", json=request)
        assert response.status_code == 404


class TestTracesEndpoint:
    """Tests for /v1/traces endpoints."""

    def test_get_trace_events(self, client, session_data):
        """Test getting trace events."""
        trace_id = session_data["trace_id"]

        response = client.get(f"/v1/traces/{trace_id}/events")
        assert response.status_code == 200

        data = response.json()
        assert data["trace_id"] == trace_id
        assert "events" in data
        assert "total_count" in data

        # Should have at least the session started event
        assert len(data["events"]) >= 1
