"""Tests for the user-managed property schema models (models/schema.py)."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from eduport.models import (
    CheckboxProperty,
    DateProperty,
    EntitySchema,
    EntityType,
    MultiSelectProperty,
    NumberProperty,
    RelationProperty,
    Schema,
    SelectOption,
    SingleSelectProperty,
    TextProperty,
    UrlProperty,
    empty_schema,
)
from eduport.models.schema import SCHEMA_VERSION


class TestKeyValidation:
    @pytest.mark.parametrize("key", ["tier", "gpa_required", "a", "tier_2", "x" * 64])
    def test_valid_keys(self, key: str) -> None:
        TextProperty(type="text", key=key, name="Tier")

    @pytest.mark.parametrize(
        "key",
        [
            "",  # empty
            "Tier",  # uppercase
            "1tier",  # leading digit
            "_tier",  # leading underscore
            "tier-1",  # hyphen
            "tier value",  # space
            "x" * 65,  # too long
        ],
    )
    def test_invalid_keys(self, key: str) -> None:
        with pytest.raises(ValidationError):
            TextProperty(type="text", key=key, name="Tier")


class TestSelectOptions:
    def test_unique_option_values(self) -> None:
        with pytest.raises(ValidationError, match="duplicate option value"):
            SingleSelectProperty(
                type="single-select",
                key="tier",
                name="Tier",
                options=[
                    SelectOption(value="reach", label="Reach"),
                    SelectOption(value="reach", label="Reach 2"),
                ],
            )

    def test_default_must_be_in_options(self) -> None:
        with pytest.raises(ValidationError, match="default"):
            SingleSelectProperty(
                type="single-select",
                key="tier",
                name="Tier",
                options=[SelectOption(value="reach", label="Reach")],
                default="safety",
            )

    def test_valid_default(self) -> None:
        prop = SingleSelectProperty(
            type="single-select",
            key="tier",
            name="Tier",
            options=[
                SelectOption(value="reach", label="Reach", color="red"),
                SelectOption(value="safety", label="Safety", color="green"),
            ],
            default="safety",
        )
        assert prop.default == "safety"

    def test_multi_select_default_subset(self) -> None:
        with pytest.raises(ValidationError, match="default"):
            MultiSelectProperty(
                type="multi-select",
                key="cats",
                name="Categories",
                options=[SelectOption(value="a", label="A")],
                default=["a", "b"],
            )

    def test_multi_select_valid(self) -> None:
        prop = MultiSelectProperty(
            type="multi-select",
            key="cats",
            name="Categories",
            options=[
                SelectOption(value="a", label="A"),
                SelectOption(value="b", label="B"),
            ],
            default=["a", "b"],
        )
        assert prop.default == ["a", "b"]

    def test_invalid_color(self) -> None:
        with pytest.raises(ValidationError):
            SelectOption(value="x", label="X", color="chartreuse")  # type: ignore[arg-type]

    def test_invalid_option_value_shape(self) -> None:
        with pytest.raises(ValidationError, match="option value"):
            SelectOption(value="X 1", label="X")


class TestPerTypeProperties:
    def test_number_default(self) -> None:
        p = NumberProperty(type="number", key="gpa", name="GPA", unit="/4.0", default=3.5)
        assert p.unit == "/4.0"
        assert p.default == 3.5

    def test_date_default_iso(self) -> None:
        p = DateProperty(type="date", key="deadline", name="Deadline", default="2026-01-15")
        assert p.default == "2026-01-15"

    def test_date_default_invalid(self) -> None:
        with pytest.raises(ValidationError, match="ISO date"):
            DateProperty(type="date", key="deadline", name="Deadline", default="15/01/2026")

    def test_checkbox(self) -> None:
        p = CheckboxProperty(type="checkbox", key="visa_required", name="Visa", default=True)
        assert p.default is True

    def test_url_default(self) -> None:
        p = UrlProperty(type="url", key="program_page", name="Program page", default="https://example.com")  # type: ignore[arg-type]
        assert str(p.default).startswith("https://example.com")

    def test_url_default_invalid(self) -> None:
        with pytest.raises(ValidationError):
            UrlProperty(type="url", key="x", name="X", default="not a url")  # type: ignore[arg-type]

    def test_relation_target_types(self) -> None:
        p = RelationProperty(
            type="relation",
            key="advisor",
            name="Advisor",
            target_types=[EntityType.person, EntityType.lab],
        )
        assert p.target_types == [EntityType.person, EntityType.lab]

    def test_relation_empty_target_types_rejected(self) -> None:
        with pytest.raises(ValidationError, match="target_types"):
            RelationProperty(type="relation", key="x", name="X", target_types=[])

    def test_relation_target_types_omitted_means_any(self) -> None:
        p = RelationProperty(type="relation", key="x", name="X")
        assert p.target_types is None


class TestEntitySchema:
    def test_unique_keys_within_type(self) -> None:
        with pytest.raises(ValidationError, match="duplicate property key"):
            EntitySchema(
                properties=[
                    TextProperty(type="text", key="x", name="A"),
                    NumberProperty(type="number", key="x", name="B"),
                ]
            )

    def test_property_lookup(self) -> None:
        schema = EntitySchema(
            properties=[
                TextProperty(type="text", key="notes", name="Notes"),
            ]
        )
        assert schema.property("notes") is not None
        assert schema.property("missing") is None


class TestFullSchema:
    def test_empty_schema_has_all_types(self) -> None:
        schema = empty_schema()
        assert set(schema.types.keys()) == set(EntityType)
        for entity_schema in schema.types.values():
            assert entity_schema.properties == []

    def test_missing_entity_type_rejected(self) -> None:
        with pytest.raises(ValidationError, match="missing entries"):
            Schema(
                version=SCHEMA_VERSION,
                types={EntityType.university: EntitySchema(properties=[])},
            )

    def test_unknown_extra_field_rejected(self) -> None:
        with pytest.raises(ValidationError):
            Schema.model_validate(
                {
                    "version": SCHEMA_VERSION,
                    "types": {t.value: {"properties": []} for t in EntityType},
                    "rogue_field": "no",
                }
            )

    def test_unsupported_version_rejected(self) -> None:
        with pytest.raises(ValidationError, match="unsupported schema version"):
            Schema.model_validate(
                {"version": 999, "types": {t.value: {"properties": []} for t in EntityType}}
            )

    def test_round_trip_yaml_shape(self) -> None:
        """Schema should serialize to a shape that round-trips back."""
        original = Schema(
            version=SCHEMA_VERSION,
            types={
                t: EntitySchema(properties=[]) for t in EntityType
            }
            | {
                EntityType.university: EntitySchema(
                    properties=[
                        SingleSelectProperty(
                            type="single-select",
                            key="tier",
                            name="Tier",
                            options=[
                                SelectOption(value="reach", label="Reach", color="red"),
                                SelectOption(value="safety", label="Safety", color="green"),
                            ],
                        ),
                    ]
                ),
            },
        )
        dumped = original.model_dump(mode="json")
        restored = Schema.model_validate(dumped)
        assert restored == original

    def test_for_type_returns_entity_schema(self) -> None:
        schema = empty_schema()
        assert schema.for_type(EntityType.university).properties == []


class TestDiscriminatedUnion:
    def test_validation_via_dict(self) -> None:
        """Property union should pick the right type based on `type`."""
        schema = EntitySchema.model_validate(
            {
                "properties": [
                    {
                        "type": "single-select",
                        "key": "tier",
                        "name": "Tier",
                        "options": [
                            {"value": "reach", "label": "Reach", "color": "red"},
                        ],
                    },
                    {
                        "type": "number",
                        "key": "gpa",
                        "name": "GPA",
                        "unit": "/4.0",
                    },
                ]
            }
        )
        assert isinstance(schema.properties[0], SingleSelectProperty)
        assert isinstance(schema.properties[1], NumberProperty)

    def test_unknown_type_rejected(self) -> None:
        with pytest.raises(ValidationError):
            EntitySchema.model_validate(
                {"properties": [{"type": "wizardry", "key": "x", "name": "X"}]}
            )
