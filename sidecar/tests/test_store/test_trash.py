from pathlib import Path

from eduport.store.trash import LocalTrash


def test_local_trash_moves_to_subfolder(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f = folder / "eth-K9p3.md"
    f.write_text("body")

    trash = LocalTrash(folder)
    moved = trash.trash(f)
    assert not f.exists()
    assert moved.exists()
    assert moved.parent.name == ".eduport-trash"
    assert moved.read_text() == "body"


def test_local_trash_handles_name_collision(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f1 = folder / "eth.md"
    f1.write_text("v1")

    trash = LocalTrash(folder)
    first = trash.trash(f1)

    f2 = folder / "eth.md"
    f2.write_text("v2")
    second = trash.trash(f2)

    assert first.exists()
    assert second.exists()
    assert first != second
    assert {first.read_text(), second.read_text()} == {"v1", "v2"}


def test_restore_returns_to_original(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f = folder / "eth.md"
    f.write_text("body")
    trash = LocalTrash(folder)
    moved = trash.trash(f)

    restored = trash.restore(moved)
    assert restored == f
    assert restored.exists()
    assert not moved.exists()
