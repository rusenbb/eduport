import pytest
from pydantic import ValidationError

from eduport.models.base import (
    BaseEntity,
    EmailResource,
    EntityType,
    LinkResource,
    WikiLink,
)


def test_wikilink_accepts_bracketed_string():
    link = WikiLink.model_validate("[[jane-doe-A4f2]]")
    assert link.target == "jane-doe-A4f2"


def test_wikilink_rejects_unbracketed():
    with pytest.raises(ValidationError):
        WikiLink.model_validate("jane-doe-A4f2")


def test_link_resource_round_trip():
    raw = {"label": "Program page", "url": "https://example.com"}
    parsed = LinkResource.model_validate(raw)
    assert parsed.label == "Program page"
    assert str(parsed.url) == "https://example.com/"


def test_email_resource_with_optional_person():
    raw = {
        "label": "Track lead",
        "email": "jane@example.com",
        "person": "[[jane-doe-A4f2]]",
    }
    parsed = EmailResource.model_validate(raw)
    assert parsed.email == "jane@example.com"
    assert parsed.person and parsed.person.target == "jane-doe-A4f2"


def test_base_entity_required_tags_include_type():
    obj = BaseEntity.model_validate(
        {"tags": ["eduport-type/program", "ai"], "name": "Some name"}
    )
    assert obj.entity_type() == EntityType.program
    assert "ai" in obj.user_tags()


def test_base_entity_rejects_unknown_field():
    with pytest.raises(ValidationError):
        BaseEntity.model_validate(
            {"tags": ["eduport-type/program"], "name": "X", "bogus_field": 1}
        )
