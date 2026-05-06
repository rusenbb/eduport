import pytest

from eduport.parsers.frontmatter import FrontmatterError, split


def test_split_basic():
    raw = """---
name: ETH
tags: [eduport-type/university]
---

Body text here.
"""
    fm, body = split(raw)
    assert fm == {"name": "ETH", "tags": ["eduport-type/university"]}
    assert body.strip() == "Body text here."


def test_split_no_frontmatter():
    raw = "Just a body, no frontmatter."
    fm, body = split(raw)
    assert fm == {}
    assert body == raw


def test_split_invalid_yaml_raises():
    raw = """---
name: ETH
tags: [unclosed
---

Body
"""
    with pytest.raises(FrontmatterError):
        split(raw)


def test_empty_frontmatter():
    raw = """---
---

Body
"""
    fm, body = split(raw)
    assert fm == {}
    assert body.strip() == "Body"
