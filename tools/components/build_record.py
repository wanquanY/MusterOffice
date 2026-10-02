"""Portable build evidence without changing embedded source path segments."""
from pathlib import Path


def relative_paths(value, root: Path):
    """Relativize whole path values, never replace text inside serialized JSON.

    Compiler flags and diagnostics stay verbatim. Absolute tool paths outside
    the repository stay absolute so their recorded identity is not obscured.
    """
    if isinstance(value, dict):
        return {key: relative_paths(item, root) for key, item in value.items()}
    if isinstance(value, list):
        return [relative_paths(item, root) for item in value]
    if isinstance(value, str):
        if value == str(root):
            return "."
        prefix = str(root) + "/"
        if value.startswith(prefix):
            return value[len(prefix):]
    return value
