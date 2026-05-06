
from eduport.index.writer import upsert_entity
from eduport.models import University


def test_search_finds_in_body(client, conn, settings):
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    upsert_entity(conn, "eth-K9p3", settings.data_folder / "x.md", 1, eth, "Strong AI track here")
    response = client.get("/search?q=track")
    assert response.status_code == 200
    hits = response.json()
    assert len(hits) == 1
    assert hits[0]["file_id"] == "eth-K9p3"
    assert "snippet" in hits[0]


def test_search_no_results(client):
    response = client.get("/search?q=zzznothing")
    assert response.status_code == 200
    assert response.json() == []
