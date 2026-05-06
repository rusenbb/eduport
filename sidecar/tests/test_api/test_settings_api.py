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
