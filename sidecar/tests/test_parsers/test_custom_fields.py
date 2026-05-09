"""Tests for the lenient custom-field validator (parsers/custom_fields.py)."""

from __future__ import annotations


from eduport.models import (
    CheckboxProperty,
    DateProperty,
    EntityType,
    MultiSelectProperty,
    NumberProperty,
    RelationProperty,
    SelectOption,
    SingleSelectProperty,
    TextProperty,
    University,
    UrlProperty,
    empty_schema,
)
from eduport.models.schema import EntitySchema
from eduport.parsers.custom_fields import (
    ValueWarning,
    WarningKind,
    validate_custom_fields,
)


def _schema_with(entity_type: EntityType, properties: list) -> object:
    schema = empty_schema()
    schema.types[entity_type] = EntitySchema(properties=properties)
    return schema


def _university(**extras) -> University:
    return University.model_validate(
        {
            "name": "Test U",
            "country": "X",
            "tags": ["eduport-type/university"],
            **extras,
        }
    )


class TestNoCustomFields:
    def test_clean_entity_no_warnings(self) -> None:
        schema = empty_schema()
        ent = _university()
        assert validate_custom_fields(ent, schema) == []


class TestOrphanedKeys:
    def test_undeclared_key_orphaned(self) -> None:
        schema = empty_schema()
        ent = _university(rogue="x")
        warnings = validate_custom_fields(ent, schema)
        assert len(warnings) == 1
        assert warnings[0].kind is WarningKind.orphaned
        assert warnings[0].key == "rogue"


class TestRequiredMissing:
    def test_required_missing_warning(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [TextProperty(type="text", key="motto", name="Motto", required=True)],
        )
        ent = _university()
        warnings = validate_custom_fields(ent, schema)
        assert any(w.kind is WarningKind.required_missing for w in warnings)

    def test_required_present_no_warning(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [TextProperty(type="text", key="motto", name="Motto", required=True)],
        )
        ent = _university(motto="Veritas")
        warnings = validate_custom_fields(ent, schema)
        assert warnings == []


class TestTextNumberDateCheckboxUrl:
    def test_text_accepts_string(self) -> None:
        schema = _schema_with(
            EntityType.university, [TextProperty(type="text", key="x", name="X")]
        )
        assert validate_custom_fields(_university(x="hi"), schema) == []

    def test_text_rejects_number(self) -> None:
        schema = _schema_with(
            EntityType.university, [TextProperty(type="text", key="x", name="X")]
        )
        warnings = validate_custom_fields(_university(x=42), schema)
        assert warnings[0].kind is WarningKind.type_mismatch

    def test_number_accepts_int_and_float(self) -> None:
        schema = _schema_with(
            EntityType.university, [NumberProperty(type="number", key="n", name="N")]
        )
        assert validate_custom_fields(_university(n=3), schema) == []
        assert validate_custom_fields(_university(n=3.5), schema) == []

    def test_number_rejects_bool(self) -> None:
        # bool is a subclass of int in Python — guard against accidental
        # acceptance.
        schema = _schema_with(
            EntityType.university, [NumberProperty(type="number", key="n", name="N")]
        )
        warnings = validate_custom_fields(_university(n=True), schema)
        assert warnings[0].kind is WarningKind.type_mismatch

    def test_date_accepts_iso_string(self) -> None:
        schema = _schema_with(
            EntityType.university, [DateProperty(type="date", key="d", name="D")]
        )
        assert validate_custom_fields(_university(d="2026-05-09"), schema) == []

    def test_date_rejects_garbage_string(self) -> None:
        schema = _schema_with(
            EntityType.university, [DateProperty(type="date", key="d", name="D")]
        )
        warnings = validate_custom_fields(_university(d="not a date"), schema)
        assert warnings[0].kind is WarningKind.type_mismatch

    def test_checkbox_accepts_bool_only(self) -> None:
        schema = _schema_with(
            EntityType.university, [CheckboxProperty(type="checkbox", key="b", name="B")]
        )
        assert validate_custom_fields(_university(b=True), schema) == []
        warnings = validate_custom_fields(_university(b=1), schema)
        assert warnings[0].kind is WarningKind.type_mismatch

    def test_url_validates(self) -> None:
        schema = _schema_with(
            EntityType.university, [UrlProperty(type="url", key="u", name="U")]
        )
        assert (
            validate_custom_fields(_university(u="https://example.com"), schema) == []
        )
        warnings = validate_custom_fields(_university(u="not a url"), schema)
        assert warnings[0].kind is WarningKind.type_mismatch


class TestSelects:
    def test_single_select_accepts_known_value(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[
                        SelectOption(value="reach", label="Reach"),
                        SelectOption(value="safety", label="Safety"),
                    ],
                )
            ],
        )
        assert validate_custom_fields(_university(tier="reach"), schema) == []

    def test_single_select_rejects_unknown_value(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[SelectOption(value="reach", label="Reach")],
                )
            ],
        )
        warnings = validate_custom_fields(_university(tier="rogue"), schema)
        assert warnings[0].kind is WarningKind.out_of_options

    def test_multi_select_partial_invalid(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [
                MultiSelectProperty(
                    type="multi-select",
                    key="cats",
                    name="Categories",
                    options=[
                        SelectOption(value="a", label="A"),
                        SelectOption(value="b", label="B"),
                    ],
                )
            ],
        )
        warnings = validate_custom_fields(_university(cats=["a", "c"]), schema)
        assert warnings[0].kind is WarningKind.out_of_options
        assert warnings[0].value == ["c"]

    def test_multi_select_wrong_shape(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [
                MultiSelectProperty(
                    type="multi-select",
                    key="cats",
                    name="Categories",
                    options=[SelectOption(value="a", label="A")],
                )
            ],
        )
        warnings = validate_custom_fields(_university(cats="a"), schema)
        assert warnings[0].kind is WarningKind.type_mismatch


class TestRelation:
    def test_valid_link_known_target(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [RelationProperty(type="relation", key="advisor", name="Advisor")],
        )
        ent = _university(advisor="[[jane-doe-A1B2]]")
        warnings = validate_custom_fields(
            ent, schema, known_target_ids={"jane-doe-A1B2": EntityType.person}
        )
        assert warnings == []

    def test_broken_link(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [RelationProperty(type="relation", key="advisor", name="Advisor")],
        )
        ent = _university(advisor="[[ghost-XXXX]]")
        warnings = validate_custom_fields(ent, schema, known_target_ids={})
        assert warnings[0].kind is WarningKind.broken_link

    def test_wrong_target_type(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [
                RelationProperty(
                    type="relation",
                    key="advisor",
                    name="Advisor",
                    target_types=[EntityType.person],
                )
            ],
        )
        ent = _university(advisor="[[some-lab-LL11]]")
        warnings = validate_custom_fields(
            ent, schema, known_target_ids={"some-lab-LL11": EntityType.lab}
        )
        assert warnings[0].kind is WarningKind.wrong_target_type

    def test_invalid_string_shape(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [RelationProperty(type="relation", key="advisor", name="Advisor")],
        )
        ent = _university(advisor="jane-doe")  # not a wikilink
        warnings = validate_custom_fields(ent, schema)
        assert warnings[0].kind is WarningKind.type_mismatch

    def test_no_link_check_when_targets_omitted(self) -> None:
        schema = _schema_with(
            EntityType.university,
            [RelationProperty(type="relation", key="advisor", name="Advisor")],
        )
        ent = _university(advisor="[[anyone-Z9Z9]]")
        # known_target_ids=None ⇒ no broken-link check
        warnings = validate_custom_fields(ent, schema)
        assert warnings == []


class TestSerialization:
    def test_warning_to_dict(self) -> None:
        from eduport.parsers.custom_fields import warning_to_dict

        w = ValueWarning(
            key="tier",
            kind=WarningKind.out_of_options,
            message="not in options",
            value="rogue",
        )
        assert warning_to_dict(w) == {
            "key": "tier",
            "kind": "out_of_options",
            "message": "not in options",
            "value": "rogue",
        }

    def test_warning_to_dict_omits_none_value(self) -> None:
        from eduport.parsers.custom_fields import warning_to_dict

        w = ValueWarning(key="x", kind=WarningKind.required_missing, message="missing")
        assert warning_to_dict(w) == {
            "key": "x",
            "kind": "required_missing",
            "message": "missing",
        }


def test_extra_keys_round_trip_through_pydantic() -> None:
    """Sanity: relaxed entity model preserves extras for the validator to see."""
    ent = University.model_validate(
        {
            "name": "Test U",
            "country": "X",
            "tags": ["eduport-type/university"],
            "tier": "reach",
            "ranking": 12,
        }
    )
    assert ent.model_extra == {"tier": "reach", "ranking": 12}
