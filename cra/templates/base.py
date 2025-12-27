"""Base template generator interface."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from cra.core.atlas import LoadedAtlas
from cra.core.carp import Resolution


@dataclass
class GeneratedFile:
    """A generated file from a template."""

    path: str
    content: str
    executable: bool = False
    description: str = ""


@dataclass
class GeneratedTemplate:
    """Result of template generation."""

    framework: str
    files: list[GeneratedFile] = field(default_factory=list)
    instructions: str = ""
    dependencies: list[str] = field(default_factory=list)

    def write_to_directory(self, output_dir: Path) -> list[Path]:
        """Write all generated files to a directory.

        Args:
            output_dir: Directory to write files to

        Returns:
            List of written file paths
        """
        written = []
        output_dir.mkdir(parents=True, exist_ok=True)

        for gen_file in self.files:
            file_path = output_dir / gen_file.path
            file_path.parent.mkdir(parents=True, exist_ok=True)
            file_path.write_text(gen_file.content)

            if gen_file.executable:
                file_path.chmod(0o755)

            written.append(file_path)

        return written


class TemplateGenerator(ABC):
    """Base class for agent template generators."""

    @property
    @abstractmethod
    def framework_name(self) -> str:
        """Name of the target framework."""
        pass

    @property
    @abstractmethod
    def framework_version(self) -> str:
        """Minimum supported version of the framework."""
        pass

    @abstractmethod
    def generate(
        self,
        atlas: LoadedAtlas,
        resolution: Resolution | None = None,
        config: dict[str, Any] | None = None,
    ) -> GeneratedTemplate:
        """Generate agent template files.

        Args:
            atlas: The Atlas to generate from
            resolution: Optional pre-computed resolution
            config: Optional generation configuration

        Returns:
            Generated template with files and instructions
        """
        pass

    def _get_atlas_actions(self, atlas: LoadedAtlas) -> list[dict[str, Any]]:
        """Extract actions from Atlas manifest."""
        actions = []
        for adapter_name, adapter_config in atlas.manifest.adapters.model_dump().items():
            if adapter_config:
                # In real impl, would load adapter files and extract actions
                pass
        return actions

    def _generate_requirements(self, extra_deps: list[str] | None = None) -> str:
        """Generate requirements.txt content."""
        base_deps = [
            "cra>=0.1.0",
            "httpx>=0.25.0",
        ]
        if extra_deps:
            base_deps.extend(extra_deps)
        return "\n".join(sorted(set(base_deps)))
