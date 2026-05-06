from datetime import date

from eduport.parsers.checkboxes import Checkbox, parse


def test_parse_unchecked():
    body = "- [ ] Buy groceries"
    items = parse(body)
    assert items == [Checkbox(line=0, checked=False, text="Buy groceries", deadline=None)]


def test_parse_checked():
    body = "- [x] Done"
    items = parse(body)
    assert items[0].checked is True


def test_parse_with_inline_date():
    body = "- [ ] Submit by 2026-12-15"
    items = parse(body)
    assert items[0].deadline == date(2026, 12, 15)
    assert items[0].text == "Submit by 2026-12-15"


def test_indented_checkbox_ignored():
    body = "  - [ ] Sub-item not parsed"
    items = parse(body)
    assert items == []


def test_multiple_lines():
    body = "- [x] Done\nplain text\n- [ ] Todo by 2027-01-01"
    items = parse(body)
    assert len(items) == 2
    assert items[0].line == 0
    assert items[1].line == 2
