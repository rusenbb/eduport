import secrets
from typing import Callable

_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
_LENGTH = 4
_MAX_RETRIES = 100


def generate_id(exists: Callable[[str], bool]) -> str:
    """Generate a fresh 4-char alphanumeric id. Retries on collision via `exists`."""
    for _ in range(_MAX_RETRIES):
        candidate = "".join(secrets.choice(_ALPHABET) for _ in range(_LENGTH))
        if not exists(candidate):
            return candidate
    raise RuntimeError(f"Could not generate unique id after {_MAX_RETRIES} attempts")
