import re
from dataclasses import dataclass
from datetime import date
from typing import Optional

_CHECKBOX_RE = re.compile(r"^- \[( |x|X)\] (.+)$")
_DATE_RE = re.compile(r"\b(\d{4}-\d{2}-\d{2})\b")


@dataclass(frozen=True)
class Checkbox:
    line: int
    checked: bool
    text: str
    deadline: Optional[date]


def parse(body: str) -> list[Checkbox]:
    items: list[Checkbox] = []
    for line_no, line in enumerate(body.splitlines()):
        m = _CHECKBOX_RE.match(line)
        if not m:
            continue
        checked = m.group(1).lower() == "x"
        text = m.group(2).strip()
        deadline: Optional[date] = None
        date_match = _DATE_RE.search(text)
        if date_match:
            try:
                deadline = date.fromisoformat(date_match.group(1))
            except ValueError:
                pass
        items.append(Checkbox(line=line_no, checked=checked, text=text, deadline=deadline))
    return items
