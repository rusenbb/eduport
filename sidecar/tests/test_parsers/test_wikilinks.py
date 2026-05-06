from eduport.parsers.wikilinks import extract_targets, resolve


def test_extract_from_string_value():
    assert extract_targets("[[jane-A4f2]]") == ["jane-A4f2"]
    assert extract_targets("plain string") == []


def test_extract_from_nested_structure():
    payload = {
        "university": "[[eth-K9p3]]",
        "people": ["[[a-1111]]", "[[b-2222]]"],
        "emails": [{"label": "x", "email": "y", "person": "[[c-3333]]"}],
    }
    found = sorted(extract_targets(payload))
    assert found == ["a-1111", "b-2222", "c-3333", "eth-K9p3"]


def test_resolve_exact_match():
    candidates = ["jane-doe-A4f2", "bob-K9p3", "msc-cs-Q7w8"]
    assert resolve("jane-doe-A4f2", candidates) == "jane-doe-A4f2"


def test_resolve_id_suffix_fallback():
    candidates = ["jane-doe-renamed-A4f2", "bob-K9p3"]
    assert resolve("jane-doe-A4f2", candidates) == "jane-doe-renamed-A4f2"


def test_resolve_broken_returns_none():
    assert resolve("ghost-Z9z9", ["jane-A4f2"]) is None


def test_resolve_ambiguous_id_picks_none():
    # Two candidates share the same id suffix — that's a vault corruption
    candidates = ["a-A4f2", "b-A4f2"]
    assert resolve("c-A4f2", candidates) is None
