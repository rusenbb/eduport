import re

import pytest

from eduport.ids import generate_id

_PATTERN = re.compile(r"^[a-zA-Z0-9]{4}$")


def test_id_shape():
    assert _PATTERN.match(generate_id(lambda _id: False))


def test_collision_retried():
    seen: list[str] = []

    def exists(candidate: str) -> bool:
        if len(seen) < 3:
            seen.append(candidate)
            return True  # collide three times
        return False

    result = generate_id(exists)
    assert _PATTERN.match(result)
    assert len(seen) == 3
    assert result not in seen


def test_unique_across_many_calls():
    used: set[str] = set()

    def exists(candidate: str) -> bool:
        return candidate in used

    for _ in range(200):
        new_id = generate_id(exists)
        used.add(new_id)

    assert len(used) == 200


def test_exhaustion_raises_runtime_error():
    with pytest.raises(RuntimeError, match="100 attempts"):
        generate_id(lambda _candidate: True)
