from datetime import date
from pathlib import Path

from eduport.models import EmailDirection
from eduport.parsers.eml import parse_eml


def test_parse_sample_eml():
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    parsed = parse_eml(fixture.read_bytes(), user_email="rusen@example.com")
    assert parsed.from_ == "jane@example.com"
    assert parsed.to == ["rusen@example.com"]
    assert parsed.cc == ["bob@example.com"]
    assert parsed.subject == "Welcome to ETH"
    assert parsed.date == date(2026, 9, 20)
    assert parsed.direction == EmailDirection.inbound
    assert "Welcome to the program" in parsed.body


def test_outbound_inferred_when_user_in_from():
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    parsed = parse_eml(fixture.read_bytes(), user_email="jane@example.com")
    assert parsed.direction == EmailDirection.outbound
