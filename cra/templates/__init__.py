"""Agent templates and framework integrations.

Templates provide scaffolding for various agent frameworks
that integrate with CRA governance.
"""

from cra.templates.base import TemplateGenerator, GeneratedTemplate
from cra.templates.openai_gpt import OpenAIGPTActionGenerator
from cra.templates.langchain import LangChainGenerator
from cra.templates.crewai import CrewAIGenerator

__all__ = [
    "TemplateGenerator",
    "GeneratedTemplate",
    "OpenAIGPTActionGenerator",
    "LangChainGenerator",
    "CrewAIGenerator",
]


def get_template_generator(framework: str) -> TemplateGenerator:
    """Get a template generator for a framework.

    Args:
        framework: Framework name (openai_gpt, langchain, crewai)

    Returns:
        The template generator instance

    Raises:
        ValueError: If framework not supported
    """
    generators = {
        "openai_gpt": OpenAIGPTActionGenerator,
        "langchain": LangChainGenerator,
        "crewai": CrewAIGenerator,
    }

    if framework not in generators:
        raise ValueError(
            f"Unknown framework: {framework}. "
            f"Supported: {', '.join(generators.keys())}"
        )

    return generators[framework]()
