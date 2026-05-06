
from eduport.index.writer import upsert_entity
from eduport.models import Application


def test_toggle_checkbox(client, conn, settings):
    body = "Note\n- [ ] task one\n- [ ] task two\n"
    app_entity = Application.model_validate({
        "tags": ["eduport-type/application"],
        "name": "ETH 2026",
        "program": "[[msc-cs-Q7w8]]",
        "status": "drafting",
    })
    path = settings.data_folder / "app-A1a1.md"
    path.write_text(
        f"---\ntags: [eduport-type/application]\nname: ETH 2026\n"
        f'program: "[[msc-cs-Q7w8]]"\nstatus: drafting\n---\n\n{body}',
        encoding="utf-8",
    )
    upsert_entity(conn, "app-A1a1", path, path.stat().st_mtime_ns, app_entity, body)

    response = client.post(
        "/checkbox/toggle",
        json={"file_id": "app-A1a1", "line": 1, "checked": True},
    )
    assert response.status_code == 200

    new_text = path.read_text(encoding="utf-8")
    assert "- [x] task one" in new_text
    assert "- [ ] task two" in new_text
