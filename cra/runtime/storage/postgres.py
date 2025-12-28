"""PostgreSQL storage implementations for production."""

import asyncio
import json
import os
from datetime import datetime, timezone
from typing import Any, AsyncIterator
from uuid import UUID

from cra.core.trace import TraceEvent, TraceContext, Severity, Actor, ActorType
from cra.core.session import Session, Principal, PrincipalType
from cra.runtime.storage.base import TraceStore, SessionStore


class PostgresTraceStore(TraceStore):
    """PostgreSQL trace storage for production.

    Provides durable, queryable trace storage with:
    - Append-only event log
    - Efficient indexing by trace_id, session_id, time
    - LISTEN/NOTIFY for real-time streaming
    """

    def __init__(
        self,
        connection_string: str | None = None,
        pool_size: int = 10,
    ):
        self.connection_string = connection_string or os.getenv(
            "CRA_DATABASE_URL",
            "postgresql://localhost:5432/cra",
        )
        self.pool_size = pool_size
        self._pool: Any = None
        self._initialized = False

    async def _ensure_pool(self) -> Any:
        """Ensure database pool is initialized."""
        if self._pool is None:
            try:
                import asyncpg
                self._pool = await asyncpg.create_pool(
                    self.connection_string,
                    min_size=2,
                    max_size=self.pool_size,
                )
            except ImportError:
                raise RuntimeError(
                    "asyncpg is required for PostgreSQL storage. "
                    "Install with: pip install asyncpg"
                )

        if not self._initialized:
            await self._initialize_schema()
            self._initialized = True

        return self._pool

    async def _initialize_schema(self) -> None:
        """Initialize database schema."""
        pool = self._pool
        async with pool.acquire() as conn:
            await conn.execute("""
                CREATE TABLE IF NOT EXISTS trace_events (
                    id SERIAL PRIMARY KEY,
                    trace_id UUID NOT NULL,
                    span_id UUID NOT NULL,
                    parent_span_id UUID,
                    session_id UUID NOT NULL,
                    event_type VARCHAR(255) NOT NULL,
                    severity VARCHAR(20) NOT NULL,
                    actor_type VARCHAR(50) NOT NULL,
                    actor_id VARCHAR(255) NOT NULL,
                    payload JSONB,
                    artifacts JSONB,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

                    -- Indexes
                    CONSTRAINT trace_events_trace_id_idx
                        UNIQUE (trace_id, id)
                );

                CREATE INDEX IF NOT EXISTS trace_events_session_idx
                    ON trace_events (session_id);
                CREATE INDEX IF NOT EXISTS trace_events_time_idx
                    ON trace_events (created_at);
                CREATE INDEX IF NOT EXISTS trace_events_type_idx
                    ON trace_events (event_type);
            """)

    async def append(self, event: TraceEvent) -> None:
        """Append a trace event."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            await conn.execute(
                """
                INSERT INTO trace_events (
                    trace_id, span_id, parent_span_id, session_id,
                    event_type, severity, actor_type, actor_id,
                    payload, artifacts, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                """,
                event.trace.trace_id,
                event.trace.span_id,
                event.trace.parent_span_id,
                event.session_id,
                event.event_type,
                event.severity.value,
                event.actor.type.value,
                event.actor.id,
                json.dumps(event.payload) if event.payload else None,
                json.dumps([a.model_dump() for a in event.artifacts]) if event.artifacts else None,
                event.time,
            )

            # Notify listeners
            await conn.execute(
                "SELECT pg_notify($1, $2)",
                f"trace_{event.trace.trace_id}",
                event.model_dump_json(),
            )

    async def get_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[TraceEvent]:
        """Get events for a trace."""
        pool = await self._ensure_pool()

        query = """
            SELECT trace_id, span_id, parent_span_id, session_id,
                   event_type, severity, actor_type, actor_id,
                   payload, artifacts, created_at
            FROM trace_events
            WHERE trace_id = $1
        """
        params: list[Any] = [trace_id]
        param_idx = 2

        if event_type:
            query += f" AND event_type LIKE ${param_idx}"
            params.append(f"{event_type}%")
            param_idx += 1

        if severity:
            query += f" AND severity = ${param_idx}"
            params.append(severity)
            param_idx += 1

        query += f" ORDER BY id LIMIT ${param_idx} OFFSET ${param_idx + 1}"
        params.extend([limit, offset])

        async with pool.acquire() as conn:
            rows = await conn.fetch(query, *params)

        return [self._row_to_event(row) for row in rows]

    async def stream_events(
        self,
        trace_id: UUID,
        event_type: str | None = None,
        severity: str | None = None,
    ) -> AsyncIterator[TraceEvent]:
        """Stream events for a trace."""
        pool = await self._ensure_pool()

        # First yield existing events
        existing = await self.get_events(
            trace_id, event_type, severity, limit=10000
        )
        for event in existing:
            yield event

        # Then listen for new events
        async with pool.acquire() as conn:
            await conn.add_listener(
                f"trace_{trace_id}",
                lambda *args: None,  # Placeholder
            )

            # Use a queue for the listener
            queue: asyncio.Queue[str] = asyncio.Queue()

            def listener(conn: Any, pid: int, channel: str, payload: str) -> None:
                asyncio.create_task(queue.put(payload))

            await conn.add_listener(f"trace_{trace_id}", listener)

            try:
                while True:
                    payload = await queue.get()
                    event = TraceEvent.model_validate_json(payload)

                    if event_type and not event.event_type.startswith(event_type):
                        continue
                    if severity and event.severity.value != severity:
                        continue

                    yield event
            finally:
                await conn.remove_listener(f"trace_{trace_id}", listener)

    async def get_event_count(self, trace_id: UUID) -> int:
        """Get event count for a trace."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                "SELECT COUNT(*) as count FROM trace_events WHERE trace_id = $1",
                trace_id,
            )
            return row["count"]

    async def delete_trace(self, trace_id: UUID) -> bool:
        """Delete all events for a trace."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            result = await conn.execute(
                "DELETE FROM trace_events WHERE trace_id = $1",
                trace_id,
            )
            return "DELETE" in result

    async def get_traces(
        self,
        session_id: UUID | None = None,
        start_time: datetime | None = None,
        end_time: datetime | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        """Get trace summaries."""
        pool = await self._ensure_pool()

        query = """
            SELECT
                trace_id,
                session_id,
                COUNT(*) as event_count,
                MIN(created_at) as first_event_time,
                MAX(created_at) as last_event_time
            FROM trace_events
            WHERE 1=1
        """
        params: list[Any] = []
        param_idx = 1

        if session_id:
            query += f" AND session_id = ${param_idx}"
            params.append(session_id)
            param_idx += 1

        if start_time:
            query += f" AND created_at >= ${param_idx}"
            params.append(start_time)
            param_idx += 1

        if end_time:
            query += f" AND created_at <= ${param_idx}"
            params.append(end_time)
            param_idx += 1

        query += f"""
            GROUP BY trace_id, session_id
            ORDER BY first_event_time DESC
            LIMIT ${param_idx} OFFSET ${param_idx + 1}
        """
        params.extend([limit, offset])

        async with pool.acquire() as conn:
            rows = await conn.fetch(query, *params)

        return [
            {
                "trace_id": str(row["trace_id"]),
                "session_id": str(row["session_id"]),
                "event_count": row["event_count"],
                "first_event_time": row["first_event_time"].isoformat(),
                "last_event_time": row["last_event_time"].isoformat(),
            }
            for row in rows
        ]

    def _row_to_event(self, row: Any) -> TraceEvent:
        """Convert a database row to a TraceEvent."""
        return TraceEvent(
            trace_version="1.0",
            event_type=row["event_type"],
            time=row["created_at"],
            trace=TraceContext(
                trace_id=row["trace_id"],
                span_id=row["span_id"],
                parent_span_id=row["parent_span_id"],
            ),
            session_id=row["session_id"],
            actor=Actor(
                type=ActorType(row["actor_type"]),
                id=row["actor_id"],
            ),
            severity=Severity(row["severity"]),
            payload=json.loads(row["payload"]) if row["payload"] else None,
            artifacts=json.loads(row["artifacts"]) if row["artifacts"] else None,
        )

    async def close(self) -> None:
        """Close the connection pool."""
        if self._pool:
            await self._pool.close()
            self._pool = None


class PostgresSessionStore(SessionStore):
    """PostgreSQL session storage for production."""

    def __init__(
        self,
        connection_string: str | None = None,
        pool_size: int = 10,
    ):
        self.connection_string = connection_string or os.getenv(
            "CRA_DATABASE_URL",
            "postgresql://localhost:5432/cra",
        )
        self.pool_size = pool_size
        self._pool: Any = None
        self._initialized = False

    async def _ensure_pool(self) -> Any:
        """Ensure database pool is initialized."""
        if self._pool is None:
            try:
                import asyncpg
                self._pool = await asyncpg.create_pool(
                    self.connection_string,
                    min_size=2,
                    max_size=self.pool_size,
                )
            except ImportError:
                raise RuntimeError(
                    "asyncpg is required for PostgreSQL storage. "
                    "Install with: pip install asyncpg"
                )

        if not self._initialized:
            await self._initialize_schema()
            self._initialized = True

        return self._pool

    async def _initialize_schema(self) -> None:
        """Initialize database schema."""
        pool = self._pool
        async with pool.acquire() as conn:
            await conn.execute("""
                CREATE TABLE IF NOT EXISTS sessions (
                    session_id UUID PRIMARY KEY,
                    trace_id UUID NOT NULL,
                    principal_type VARCHAR(50) NOT NULL,
                    principal_id VARCHAR(255) NOT NULL,
                    scopes JSONB NOT NULL DEFAULT '[]',
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    expires_at TIMESTAMPTZ NOT NULL,
                    ended_at TIMESTAMPTZ,
                    metadata JSONB
                );

                CREATE INDEX IF NOT EXISTS sessions_principal_idx
                    ON sessions (principal_id);
                CREATE INDEX IF NOT EXISTS sessions_expires_idx
                    ON sessions (expires_at);
            """)

    async def create(self, session: Session) -> Session:
        """Create a new session."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            await conn.execute(
                """
                INSERT INTO sessions (
                    session_id, trace_id, principal_type, principal_id,
                    scopes, created_at, expires_at, ended_at, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                """,
                session.session_id,
                session.trace_id,
                session.principal.type.value,
                session.principal.id,
                json.dumps(session.scopes),
                session.created_at,
                session.expires_at,
                session.ended_at,
                json.dumps(session.metadata) if session.metadata else None,
            )

        return session

    async def get(self, session_id: UUID) -> Session | None:
        """Get a session by ID."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                "SELECT * FROM sessions WHERE session_id = $1",
                session_id,
            )

        if not row:
            return None

        return self._row_to_session(row)

    async def update(self, session: Session) -> Session:
        """Update a session."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            await conn.execute(
                """
                UPDATE sessions SET
                    scopes = $2,
                    expires_at = $3,
                    ended_at = $4,
                    metadata = $5
                WHERE session_id = $1
                """,
                session.session_id,
                json.dumps(session.scopes),
                session.expires_at,
                session.ended_at,
                json.dumps(session.metadata) if session.metadata else None,
            )

        return session

    async def delete(self, session_id: UUID) -> bool:
        """Delete a session."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            result = await conn.execute(
                "DELETE FROM sessions WHERE session_id = $1",
                session_id,
            )
            return "DELETE" in result

    async def list_active(
        self,
        principal_id: str | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Session]:
        """List active sessions."""
        pool = await self._ensure_pool()

        query = """
            SELECT * FROM sessions
            WHERE expires_at > NOW()
            AND ended_at IS NULL
        """
        params: list[Any] = []
        param_idx = 1

        if principal_id:
            query += f" AND principal_id = ${param_idx}"
            params.append(principal_id)
            param_idx += 1

        query += f" ORDER BY created_at DESC LIMIT ${param_idx} OFFSET ${param_idx + 1}"
        params.extend([limit, offset])

        async with pool.acquire() as conn:
            rows = await conn.fetch(query, *params)

        return [self._row_to_session(row) for row in rows]

    async def cleanup_expired(self) -> int:
        """Clean up expired sessions."""
        pool = await self._ensure_pool()

        async with pool.acquire() as conn:
            result = await conn.execute(
                """
                DELETE FROM sessions
                WHERE expires_at < NOW()
                AND ended_at IS NULL
                """
            )
            # Parse "DELETE N" to get count
            parts = result.split()
            if len(parts) >= 2:
                return int(parts[1])
            return 0

    def _row_to_session(self, row: Any) -> Session:
        """Convert a database row to a Session."""
        return Session(
            session_id=row["session_id"],
            trace_id=row["trace_id"],
            principal=Principal(
                type=PrincipalType(row["principal_type"]),
                id=row["principal_id"],
            ),
            scopes=json.loads(row["scopes"]),
            created_at=row["created_at"],
            expires_at=row["expires_at"],
            ended_at=row["ended_at"],
            metadata=json.loads(row["metadata"]) if row["metadata"] else None,
        )

    async def close(self) -> None:
        """Close the connection pool."""
        if self._pool:
            await self._pool.close()
            self._pool = None
