from __future__ import annotations

import email
from dataclasses import dataclass, field
from datetime import date
from email import policy
from email.message import EmailMessage
from email.utils import getaddresses, parsedate_to_datetime
from typing import Optional

from markdownify import markdownify

from eduport.models import EmailDirection


@dataclass
class ParsedEml:
    from_: str
    to: list[str] = field(default_factory=list)
    cc: list[str] = field(default_factory=list)
    bcc: list[str] = field(default_factory=list)
    subject: str = ""
    date: Optional[date] = None
    body: str = ""
    direction: EmailDirection = EmailDirection.inbound


def _addresses(message: EmailMessage, header: str) -> list[str]:
    raw = message.get_all(header) or []
    return [addr for _name, addr in getaddresses(raw) if addr]


def _body(message: EmailMessage) -> str:
    plain = message.get_body(preferencelist=("plain",))
    if plain is not None:
        return plain.get_content().strip()
    html_part = message.get_body(preferencelist=("html",))
    if html_part is not None:
        return markdownify(html_part.get_content()).strip()
    return ""


def parse_eml(raw: bytes, user_email: str) -> ParsedEml:
    message: EmailMessage = email.message_from_bytes(raw, policy=policy.default)  # type: ignore[assignment]

    from_addrs = _addresses(message, "From")
    from_ = from_addrs[0] if from_addrs else ""
    to = _addresses(message, "To")
    cc = _addresses(message, "Cc")
    bcc = _addresses(message, "Bcc")

    subject = message.get("Subject", "").strip()
    date_value: Optional[date] = None
    if raw_date := message.get("Date"):
        try:
            dt = parsedate_to_datetime(raw_date)
            if dt is not None:
                date_value = dt.date()
        except (TypeError, ValueError):
            date_value = None

    direction = EmailDirection.outbound if from_.lower() == user_email.lower() else EmailDirection.inbound

    return ParsedEml(
        from_=from_,
        to=to,
        cc=cc,
        bcc=bcc,
        subject=subject,
        date=date_value,
        body=_body(message),
        direction=direction,
    )
