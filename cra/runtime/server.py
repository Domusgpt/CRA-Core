"""CRA Runtime server factory."""

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from cra.version import __version__
from cra.runtime.api import (
    health_router,
    sessions_router,
    carp_router,
    traces_router,
)


def create_app() -> FastAPI:
    """Create the CRA Runtime FastAPI application.

    Returns:
        Configured FastAPI application
    """
    app = FastAPI(
        title="CRA Runtime",
        description="""
        Context Registry Agents Runtime API.

        CRA provides a governed context layer that makes AI agents use tools,
        systems, and proprietary knowledge correctly.

        ## Core Principles

        - **CARP** resolves context + permitted actions
        - **TRACE** proves what happened
        - **If it wasn't emitted by the runtime, it didn't happen**

        ## Key Endpoints

        - `POST /v1/sessions` - Create a new session
        - `POST /v1/carp/resolve` - Resolve context and actions
        - `GET /v1/traces/{trace_id}/stream` - Stream TRACE events (SSE)
        """,
        version=__version__,
        docs_url="/docs",
        redoc_url="/redoc",
    )

    # Add CORS middleware
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],  # Configure appropriately for production
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # Include routers
    app.include_router(health_router)
    app.include_router(sessions_router)
    app.include_router(carp_router)
    app.include_router(traces_router)

    return app


# Create the default app instance
app = create_app()


def run_server(host: str = "127.0.0.1", port: int = 8420) -> None:
    """Run the CRA runtime server.

    Args:
        host: Host to bind to
        port: Port to bind to
    """
    import uvicorn

    uvicorn.run(app, host=host, port=port)


if __name__ == "__main__":
    run_server()
