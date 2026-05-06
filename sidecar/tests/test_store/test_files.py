from pathlib import Path

from eduport.models import University
from eduport.store.files import (
    EntityFileStore,
    serialize_entity_to_markdown,
)


def test_serialize_entity_round_trip():
    uni = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH",
        "country": "Switzerland",
    })
    text = serialize_entity_to_markdown(uni, body="My notes")
    assert text.startswith("---")
    assert "eduport-type/university" in text
    assert text.rstrip().endswith("My notes")


def test_write_creates_file(tmp_path: Path):
    store = EntityFileStore(tmp_path)
    uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    path = store.write("eth-K9p3", uni, body="")
    assert path == tmp_path / "eth-K9p3.md"
    assert path.exists()
    assert "eduport-type/university" in path.read_text()


def test_re_entrancy_guard_marks_writes(tmp_path: Path):
    store = EntityFileStore(tmp_path)
    uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    path = store.write("eth-K9p3", uni, body="")
    assert store.was_recently_written(path) is True
    assert store.was_recently_written(path) is False
