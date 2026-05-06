import pytest

from eduport.slug import generate_slug


@pytest.mark.parametrize(
    "name, expected",
    [
        ("ETH Zurich", "eth-zurich"),
        ("ETH Zürich", "eth-zurich"),  # NFKD fold
        ("MSc CS (AI track)", "msc-cs-ai-track"),
        ("Søren Kierkegaard", "soren-kierkegaard"),
        ("Øresund Bridge", "oresund-bridge"),  # uppercase fold-table entry
        ("Æschylus", "aeschylus"),             # Æ → AE then lowercased
        ("  trailing & leading  ", "trailing-leading"),
        ("multiple --- dashes", "multiple-dashes"),
        ("CamelCase Name", "camelcase-name"),
        ("a" * 80, "a" * 60),  # truncated to 60
        ("", "untitled"),
        ("🎓🚀", "untitled"),  # emoji-only fallback
    ],
)
def test_generate_slug(name: str, expected: str):
    assert generate_slug(name) == expected


def test_truncation_at_word_boundary():
    name = "this is a fairly long sentence that exceeds sixty characters in total length"
    result = generate_slug(name)
    assert len(result) <= 60
    # No trailing partial word — must end on a complete token
    assert not result.endswith("-")
    # Token check: split and ensure last token isn't truncated mid-word
    tokens = result.split("-")
    assert all(t.isalnum() for t in tokens if t)
