from datetime import date
from enum import Enum
from typing import Optional

from eduport.models.base import BaseEntity, WikiLink


class ApplicationStatus(str, Enum):
    planning = "planning"
    drafting = "drafting"
    submitted = "submitted"
    decision_pending = "decision-pending"
    accepted = "accepted"
    rejected = "rejected"
    withdrawn = "withdrawn"


class Application(BaseEntity):
    program: WikiLink
    status: ApplicationStatus = ApplicationStatus.planning
    internal_deadline: Optional[date] = None
    submitted_at: Optional[date] = None
    decision_at: Optional[date] = None
    documents: list[WikiLink] = []
