from datetime import date
from enum import Enum
from typing import Optional

from pydantic import Field

from eduport.models.base import BaseEntity, WikiLink


class EmailDirection(str, Enum):
    inbound = "inbound"
    outbound = "outbound"


class Email(BaseEntity):
    direction: EmailDirection
    date: date
    subject: str
    from_: str = Field(alias="from")
    to: list[str] = []
    cc: list[str] = []
    bcc: list[str] = []
    related_program: Optional[WikiLink] = None
    related_application: Optional[WikiLink] = None
    related_people: list[WikiLink] = []
    in_reply_to: Optional[WikiLink] = None
    attachments: list[WikiLink] = []
