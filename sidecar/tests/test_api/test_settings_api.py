def test_get_settings(client):
    response = client.get("/settings")
    assert response.status_code == 200
    body = response.json()
    assert "data_folder" in body
    assert body["theme"] == "system"


def test_put_settings(client, settings):
    new_payload = {
        "data_folder": str(settings.data_folder),
        "attachments_folder": "./attachments",
        "notes_folder": "./notes",
        "theme": "dark",
        "user_email": "rusen@example.com",
    }
    response = client.put("/settings", json=new_payload)
    assert response.status_code == 200
    assert response.json()["theme"] == "dark"


def test_put_settings_persists_when_settings_path(settings, conn, tmp_path):
    from fastapi.testclient import TestClient

    from eduport.api.app import build_app
    from eduport.settings import load_settings

    settings_path = tmp_path / "settings.toml"
    app = build_app(
        settings=settings,
        conn=conn,
        settings_path=settings_path,
        start_watcher=False,
    )
    with TestClient(app) as c:
        response = c.put(
            "/settings",
            json={
                "data_folder": str(settings.data_folder),
                "attachments_folder": "./attachments",
                "notes_folder": "./notes",
                "theme": "dark",
                "user_email": "rusen@example.com",
            },
        )

    assert response.status_code == 200
    loaded = load_settings(settings_path)
    assert loaded is not None
    assert loaded.theme == "dark"
    assert loaded.user_email == "rusen@example.com"
