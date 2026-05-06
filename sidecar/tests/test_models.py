from datetime import date

import pytest
from pydantic import ValidationError

from eduport.models import (
    Application,
    ApplicationStatus,
    Document,
    DocumentStatus,
    Email,
    EmailDirection,
    Lab,
    Level,
    Note,
    Person,
    Program,
    University,
)
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


def test_base_entity_rejects_missing_type_tag():
    with pytest.raises(ValidationError):
        BaseEntity.model_validate({"tags": ["ai", "theory"], "name": "X"})
    with pytest.raises(ValidationError):
        BaseEntity.model_validate({"tags": [], "name": "X"})


def test_user_tags_excludes_doctype_prefix():
    obj = BaseEntity.model_validate({
        "tags": [
            "eduport-type/document",
            "eduport-doctype/cv",
            "ai",
        ],
        "name": "CV March 2026",
    })
    assert obj.user_tags() == ["ai"]


def test_university_minimal():
    u = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH Zurich",
        "country": "Switzerland",
    })
    assert u.country == "Switzerland"
    assert u.links == []
    assert u.emails == []


def test_university_with_resources():
    u = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH",
        "country": "CH",
        "links": [{"label": "Admissions", "url": "https://ethz.ch/apply"}],
        "emails": [{"label": "Info", "email": "info@ethz.ch"}],
    })
    assert len(u.links) == 1
    assert u.emails[0].email == "info@ethz.ch"


def test_lab_with_university_link():
    lab = Lab.model_validate({
        "tags": ["eduport-type/lab"],
        "name": "MLG",
        "university": "[[eth-zurich-K9p3]]",
    })
    assert lab.university and lab.university.target == "eth-zurich-K9p3"


def test_person_full():
    p = Person.model_validate({
        "tags": ["eduport-type/person", "ai"],
        "name": "Jane Doe",
        "role": "Professor",
        "email": "jane@example.com",
        "university": "[[eth-zurich-K9p3]]",
        "labs": ["[[mlg-B2n4]]"],
    })
    assert p.role == "Professor"
    assert len(p.labs) == 1


def test_program_full():
    p = Program.model_validate({
        "tags": ["eduport-type/program", "ai"],
        "name": "MSc CS",
        "level": "masters",
        "deadline": "2026-12-15",
        "university": "[[eth-zurich-K9p3]]",
        "people": ["[[jane-doe-A4f2]]"],
        "links": [{"label": "Page", "url": "https://x.example"}],
    })
    assert p.level == Level.masters
    assert p.deadline == date(2026, 12, 15)
    assert p.people[0].target == "jane-doe-A4f2"


def test_program_invalid_level_rejected():
    with pytest.raises(ValidationError):
        Program.model_validate({
            "tags": ["eduport-type/program"],
            "name": "X",
            "level": "bogus",
        })


def test_application_minimal():
    a = Application.model_validate({
        "tags": ["eduport-type/application"],
        "name": "ETH 2026",
        "program": "[[msc-cs-Q7w8]]",
        "status": "drafting",
    })
    assert a.status == ApplicationStatus.drafting
    assert a.documents == []
    assert a.submitted_at is None


def test_document_received_default():
    d = Document.model_validate({
        "tags": ["eduport-type/document", "eduport-doctype/cv"],
        "name": "CV March 2026",
        "title": "CV",
        "file": "attachments/cv.pdf",
    })
    assert d.status == DocumentStatus.received  # default when file present


def test_document_pending_recommendation():
    d = Document.model_validate({
        "tags": ["eduport-type/document", "eduport-doctype/recommendation"],
        "name": "Rec letter",
        "title": "Rec from Jane",
        "status": "requested",
        "recommender": "[[jane-doe-A4f2]]",
        "requested_at": "2026-10-01",
    })
    assert d.status == DocumentStatus.requested
    assert d.file is None
    assert d.recommender and d.recommender.target == "jane-doe-A4f2"


def test_email_full():
    e = Email.model_validate({
        "tags": ["eduport-type/email"],
        "name": "Q about deadline",
        "direction": "outbound",
        "date": "2026-09-20",
        "subject": "Question about MSc CS deadline",
        "from": "rusen@example.com",
        "to": ["admissions@inf.ethz.ch"],
        "cc": ["jane.doe@inf.ethz.ch"],
        "related_program": "[[msc-cs-Q7w8]]",
        "related_people": ["[[jane-doe-A4f2]]"],
    })
    assert e.direction == EmailDirection.outbound
    assert e.from_ == "rusen@example.com"
    assert e.cc == ["jane.doe@inf.ethz.ch"]
    assert e.related_program and e.related_program.target == "msc-cs-Q7w8"


def test_note_minimal():
    n = Note.model_validate({
        "tags": ["eduport-type/note"],
        "name": "scratchpad",
    })
    assert n.entity_type().value == "note"
