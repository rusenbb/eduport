from pathlib import Path

from eduport.models import Program, University
from eduport.parsers.entity import ParsedEntity, ParseError, parse_file


def _write(tmp_path: Path, name: str, content: str) -> Path:
    p = tmp_path / name
    p.write_text(content, encoding="utf-8")
    return p


def test_parse_university(tmp_path: Path):
    raw = """---
tags: [eduport-type/university]
name: ETH
country: Switzerland
---

Body
"""
    path = _write(tmp_path, "eth-K9p3.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParsedEntity)
    assert isinstance(result.entity, University)
    assert result.entity.name == "ETH"


def test_parse_program(tmp_path: Path):
    raw = """---
tags: [eduport-type/program]
name: MSc CS
level: masters
---
"""
    path = _write(tmp_path, "msc-Q7w8.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParsedEntity)
    assert isinstance(result.entity, Program)


def test_parse_returns_error_on_invalid_yaml(tmp_path: Path):
    raw = """---
tags: [unclosed
---
"""
    path = _write(tmp_path, "bad-X1x1.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)
    assert "yaml" in result.message.lower() or "frontmatter" in result.message.lower()


def test_parse_returns_error_on_missing_type_tag(tmp_path: Path):
    raw = """---
tags: [random]
name: X
---
"""
    path = _write(tmp_path, "x-Y2y2.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)


def test_parse_unknown_type_tag(tmp_path: Path):
    raw = """---
tags: [eduport-type/imaginary]
name: X
---
"""
    path = _write(tmp_path, "x-Z3z3.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)
