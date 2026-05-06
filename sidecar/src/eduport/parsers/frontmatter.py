from typing import Any

import yaml


class FrontmatterError(ValueError):
    pass


def split(raw: str) -> tuple[dict[str, Any], str]:
    """Return (frontmatter_dict, body_str). Empty dict if no frontmatter."""
    if not raw.startswith("---"):
        return {}, raw
    after_open = raw[3:]
    end = after_open.find("\n---")
    if end == -1:
        return {}, raw
    yaml_block = after_open[:end].strip()
    body_start = end + len("\n---")
    if body_start < len(after_open) and after_open[body_start] == "\n":
        body_start += 1
    body = after_open[body_start:]

    if not yaml_block:
        return {}, body

    try:
        parsed = yaml.safe_load(yaml_block)
    except yaml.YAMLError as exc:
        raise FrontmatterError(str(exc)) from exc

    if parsed is None:
        return {}, body
    if not isinstance(parsed, dict):
        raise FrontmatterError(f"Frontmatter must be a mapping, got {type(parsed).__name__}")
    return parsed, body
