
from eduport.index.writer import upsert_entity
from eduport.parsers.entity import ParseError, parse_file

FRONTMATTER = (
    "---\ntags: [eduport-type/application]\nname: ETH 2026\n"
    'program: "[[msc-cs-Q7w8]]"\nstatus: drafting\n---'
)


def _seed(path, conn, file_id, file_text):
    """Mirror what the watcher does: parse, then upsert with the parsed body."""
    path.write_text(file_text, encoding="utf-8")
    result = parse_file(path)
    assert not isinstance(result, ParseError)
    upsert_entity(conn, file_id, path, path.stat().st_mtime_ns, result.entity, result.body)
    return result.body


def _body_line_of(body: str, needle: str) -> int:
    """Mimic the frontend: body.split('\\n') indexed by predicate."""
    for idx, line in enumerate(body.split("\n")):
        if needle in line:
            return idx
    raise AssertionError(f"{needle!r} not in body")


def test_toggle_with_blank_line_after_frontmatter(client, conn, settings):
    """Conventional shape: blank line between '---' and body."""
    path = settings.data_folder / "app-A1a1.md"
    body = _seed(
        path, conn, "app-A1a1",
        f"{FRONTMATTER}\n\nNote\n- [ ] task one\n- [ ] task two\n",
    )
    line = _body_line_of(body, "task one")  # frontend would send this index

    response = client.post(
        "/checkbox/toggle",
        json={"file_id": "app-A1a1", "line": line, "checked": True},
    )
    assert response.status_code == 200, response.text

    new_text = path.read_text(encoding="utf-8")
    assert "- [x] task one" in new_text, "expected 'task one' to flip, not a sibling"
    assert "- [ ] task two" in new_text, "sibling must remain unchanged"


def test_toggle_without_blank_line_after_frontmatter(client, conn, settings):
    """No blank line: closing '---' immediately followed by body content."""
    path = settings.data_folder / "app-B2b2.md"
    body = _seed(
        path, conn, "app-B2b2",
        f"{FRONTMATTER}\n- [ ] only one\n",
    )
    line = _body_line_of(body, "only one")

    response = client.post(
        "/checkbox/toggle",
        json={"file_id": "app-B2b2", "line": line, "checked": True},
    )
    assert response.status_code == 200, response.text
    assert "- [x] only one" in path.read_text(encoding="utf-8")


def test_toggle_rejects_non_checkbox_line(client, conn, settings):
    path = settings.data_folder / "app-C3c3.md"
    body = _seed(
        path, conn, "app-C3c3",
        f"{FRONTMATTER}\n\nNote\n- [ ] task\n",
    )
    line = _body_line_of(body, "Note")

    response = client.post(
        "/checkbox/toggle",
        json={"file_id": "app-C3c3", "line": line, "checked": True},
    )
    assert response.status_code == 400
