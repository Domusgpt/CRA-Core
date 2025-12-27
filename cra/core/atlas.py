"""Atlas types and loader for CRA.

Atlases are creator-published packages that include:
- Manifest (atlas.json)
- Context packs
- Policies
- Adapters (Claude/OpenAI/MCP)
- Tests
"""

import json
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field


class AtlasCapability(BaseModel):
    """A capability provided by an Atlas."""

    id: str
    name: str = ""
    description: str = ""
    actions: list[str] = Field(default_factory=list)
    requires_scopes: list[str] = Field(default_factory=list)


class AtlasCertification(BaseModel):
    """Certification status for an Atlas."""

    carp_compliant: bool = False
    trace_compliant: bool = False
    last_certified: datetime | None = None
    certified_by: str | None = None
    certification_level: str = "none"  # none, basic, standard, enterprise


class AtlasLicense(str, Enum):
    """License types for Atlases."""

    MIT = "MIT"
    APACHE2 = "Apache-2.0"
    PROPRIETARY = "proprietary"
    CUSTOM = "custom"


class AtlasAdapters(BaseModel):
    """Adapter file references in an Atlas."""

    openai: str | None = None
    anthropic: str | None = None
    google_adk: str | None = None
    mcp: str | None = None


class AtlasDependency(BaseModel):
    """A dependency on another Atlas."""

    id: str
    version: str
    optional: bool = False


class AtlasManifest(BaseModel):
    """The atlas.json manifest file.

    This is the entry point for an Atlas package.
    """

    atlas_version: str = "1.0"
    id: str = Field(..., pattern=r"^[a-z][a-z0-9._-]*$")
    version: str = Field(..., pattern=r"^\d+\.\d+\.\d+")
    name: str
    description: str = ""
    author: str = ""
    homepage: str = ""
    repository: str = ""
    capabilities: list[str] = Field(default_factory=list)
    context_packs: list[str] = Field(default_factory=list)
    policies: list[str] = Field(default_factory=list)
    adapters: AtlasAdapters = Field(default_factory=AtlasAdapters)
    dependencies: list[AtlasDependency] = Field(default_factory=list)
    license: str = "MIT"
    certification: AtlasCertification = Field(default_factory=AtlasCertification)
    keywords: list[str] = Field(default_factory=list)
    min_cra_version: str = "0.1.0"


class ContextPack(BaseModel):
    """A loaded context pack from an Atlas."""

    path: str
    content_type: str
    content: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class PolicyFile(BaseModel):
    """A loaded policy file from an Atlas."""

    path: str
    policy_id: str
    name: str
    rules: list[dict[str, Any]] = Field(default_factory=list)
    defaults: dict[str, Any] = Field(default_factory=dict)


class LoadedAtlas(BaseModel):
    """A fully loaded Atlas with all resources."""

    manifest: AtlasManifest
    root_path: Path
    context_packs: list[ContextPack] = Field(default_factory=list)
    policies: list[PolicyFile] = Field(default_factory=list)
    adapters: dict[str, dict[str, Any]] = Field(default_factory=dict)

    class Config:
        arbitrary_types_allowed = True

    @property
    def id(self) -> str:
        return self.manifest.id

    @property
    def version(self) -> str:
        return self.manifest.version

    @property
    def name(self) -> str:
        return self.manifest.name

    def get_context_for_capability(self, capability: str) -> list[ContextPack]:
        """Get context packs relevant to a capability."""
        # For now, return all context packs
        # In future, filter based on capability metadata
        return self.context_packs

    def get_adapter(self, platform: str) -> dict[str, Any] | None:
        """Get adapter config for a platform."""
        return self.adapters.get(platform)


class AtlasLoadError(Exception):
    """Error loading an Atlas."""

    pass


class AtlasNotFoundError(AtlasLoadError):
    """Atlas not found."""

    pass


class AtlasValidationError(AtlasLoadError):
    """Atlas validation failed."""

    pass


class AtlasLoader:
    """Loads and validates Atlas packages."""

    def __init__(self) -> None:
        """Initialize the loader."""
        self._cache: dict[str, LoadedAtlas] = {}

    def load(self, path: Path) -> LoadedAtlas:
        """Load an Atlas from a directory.

        Args:
            path: Path to Atlas directory (containing atlas.json)

        Returns:
            LoadedAtlas with all resources

        Raises:
            AtlasNotFoundError: If atlas.json not found
            AtlasValidationError: If validation fails
        """
        path = Path(path)

        # Check cache
        cache_key = str(path.resolve())
        if cache_key in self._cache:
            return self._cache[cache_key]

        # Load manifest
        manifest_path = path / "atlas.json"
        if not manifest_path.exists():
            raise AtlasNotFoundError(f"atlas.json not found in {path}")

        try:
            with open(manifest_path) as f:
                manifest_data = json.load(f)
            manifest = AtlasManifest(**manifest_data)
        except json.JSONDecodeError as e:
            raise AtlasValidationError(f"Invalid JSON in atlas.json: {e}")
        except Exception as e:
            raise AtlasValidationError(f"Invalid atlas.json: {e}")

        # Load context packs
        context_packs = []
        for pack_path in manifest.context_packs:
            full_path = path / pack_path
            if full_path.exists():
                context_packs.append(self._load_context_pack(full_path, pack_path))

        # Load policies
        policies = []
        for policy_path in manifest.policies:
            full_path = path / policy_path
            if full_path.exists():
                policies.append(self._load_policy(full_path, policy_path))

        # Load adapters
        adapters = {}
        if manifest.adapters.openai:
            adapter_path = path / manifest.adapters.openai
            if adapter_path.exists():
                adapters["openai"] = self._load_json(adapter_path)

        if manifest.adapters.anthropic:
            adapter_path = path / manifest.adapters.anthropic
            if adapter_path.exists():
                adapters["anthropic"] = self._load_text(adapter_path)

        if manifest.adapters.google_adk:
            adapter_path = path / manifest.adapters.google_adk
            if adapter_path.exists():
                adapters["google_adk"] = self._load_json(adapter_path)

        if manifest.adapters.mcp:
            adapter_path = path / manifest.adapters.mcp
            if adapter_path.exists():
                adapters["mcp"] = self._load_json(adapter_path)

        loaded = LoadedAtlas(
            manifest=manifest,
            root_path=path,
            context_packs=context_packs,
            policies=policies,
            adapters=adapters,
        )

        # Cache it
        self._cache[cache_key] = loaded

        return loaded

    def _load_context_pack(self, path: Path, rel_path: str) -> ContextPack:
        """Load a context pack file."""
        content = path.read_text()

        # Determine content type
        suffix = path.suffix.lower()
        content_types = {
            ".md": "text/markdown",
            ".json": "application/json",
            ".txt": "text/plain",
            ".yaml": "application/yaml",
            ".yml": "application/yaml",
        }
        content_type = content_types.get(suffix, "text/plain")

        return ContextPack(
            path=rel_path,
            content_type=content_type,
            content=content,
        )

    def _load_policy(self, path: Path, rel_path: str) -> PolicyFile:
        """Load a policy file."""
        with open(path) as f:
            data = json.load(f)

        return PolicyFile(
            path=rel_path,
            policy_id=data.get("id", path.stem),
            name=data.get("name", path.stem),
            rules=data.get("rules", []),
            defaults=data.get("defaults", {}),
        )

    def _load_json(self, path: Path) -> dict[str, Any]:
        """Load a JSON file."""
        with open(path) as f:
            return json.load(f)

    def _load_text(self, path: Path) -> dict[str, Any]:
        """Load a text file as a dict with content."""
        return {"content": path.read_text(), "path": str(path)}

    def clear_cache(self) -> None:
        """Clear the loader cache."""
        self._cache.clear()

    def get_cached(self, atlas_id: str) -> LoadedAtlas | None:
        """Get a cached Atlas by ID."""
        for atlas in self._cache.values():
            if atlas.id == atlas_id:
                return atlas
        return None


class AtlasRegistry:
    """Registry of loaded Atlases."""

    def __init__(self) -> None:
        """Initialize the registry."""
        self._loader = AtlasLoader()
        self._atlases: dict[str, LoadedAtlas] = {}

    def register(self, path: Path) -> LoadedAtlas:
        """Register an Atlas from a path.

        Args:
            path: Path to Atlas directory

        Returns:
            The loaded Atlas
        """
        atlas = self._loader.load(path)
        self._atlases[atlas.id] = atlas
        return atlas

    def get(self, atlas_id: str) -> LoadedAtlas | None:
        """Get an Atlas by ID.

        Args:
            atlas_id: The Atlas ID

        Returns:
            The Atlas or None
        """
        return self._atlases.get(atlas_id)

    def get_by_capability(self, capability: str) -> list[LoadedAtlas]:
        """Get all Atlases that provide a capability.

        Args:
            capability: The capability to search for

        Returns:
            List of Atlases providing the capability
        """
        return [
            atlas
            for atlas in self._atlases.values()
            if capability in atlas.manifest.capabilities
        ]

    def list_all(self) -> list[LoadedAtlas]:
        """List all registered Atlases."""
        return list(self._atlases.values())

    def unregister(self, atlas_id: str) -> bool:
        """Unregister an Atlas.

        Args:
            atlas_id: The Atlas ID

        Returns:
            True if unregistered, False if not found
        """
        if atlas_id in self._atlases:
            del self._atlases[atlas_id]
            return True
        return False


# Global registry instance
_registry: AtlasRegistry | None = None


def get_atlas_registry() -> AtlasRegistry:
    """Get the global Atlas registry."""
    global _registry
    if _registry is None:
        _registry = AtlasRegistry()
    return _registry
