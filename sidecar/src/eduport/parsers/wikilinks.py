import re
from typing import Any, Optional, Sequence

_WIKILINK_RE = re.compile(r"\[\[([^\]\[]+)\]\]")
_ID_SUFFIX_RE = re.compile(r"-([a-zA-Z0-9]{4})$")


def extract_targets(value: Any) -> list[str]:
    """Walk a JSON-ish value and collect all wikilink targets."""
    found: list[str] = []
    _walk(value, found)
    return found


def _walk(value: Any, into: list[str]) -> None:
    if isinstance(value, str):
        for m in _WIKILINK_RE.finditer(value):
            into.append(m.group(1).strip())
    elif isinstance(value, dict):
        for v in value.values():
            _walk(v, into)
    elif isinstance(value, (list, tuple)):
        for v in value:
            _walk(v, into)


def resolve(target: str, candidates: Sequence[str]) -> Optional[str]:
    """Resolve a wikilink target against existing file stems.

    1. Exact match wins.
    2. Otherwise find a unique candidate sharing the same trailing 4-char id.
    3. Returns None if no resolution.
    """
    if target in candidates:
        return target
    m = _ID_SUFFIX_RE.search(target)
    if not m:
        return None
    id_part = m.group(1)
    matches = [c for c in candidates if c.endswith(f"-{id_part}")]
    if len(matches) == 1:
        return matches[0]
    return None
