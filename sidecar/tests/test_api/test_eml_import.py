from pathlib import Path


def test_eml_parse_returns_form_payload(client):
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    response = client.post(
        "/eml/parse",
        files={"file": ("sample.eml", fixture.read_bytes(), "message/rfc822")},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["from"] == "jane@example.com"
    assert body["subject"] == "Welcome to ETH"
    assert "Welcome to the program" in body["body"]
    assert body["direction"] == "inbound"
