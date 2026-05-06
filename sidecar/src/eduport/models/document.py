from datetime import date as _Date
from enum import Enum
from typing import Optional

from pydantic import model_validator

from eduport.models.base import BaseEntity, WikiLink


class DocumentStatus(str, Enum):
    requested = "requested"
    drafting = "drafting"
    received = "received"


class Document(BaseEntity):
    title: str
    date: Optional[_Date] = None
    file: Optional[str] = None  # path relative to data folder
    status: Optional[DocumentStatus] = None
    requested_at: Optional[_Date] = None
    recommender: Optional[WikiLink] = None

    @model_validator(mode="after")
    def _default_status(self) -> "Document":
        if self.status is None:
            self.status = DocumentStatus.received if self.file else DocumentStatus.drafting
        return self
