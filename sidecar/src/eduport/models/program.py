from datetime import date
from enum import Enum
from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, EmailResource, LinkResource, WikiLink


class Level(str, Enum):
    undergrad = "undergrad"
    masters = "masters"
    phd = "phd"


class Program(BaseEntity):
    level: Optional[Level] = None
    department: Optional[str] = None
    language: Optional[str] = None
    duration: Optional[str] = None
    deadline: Optional[date] = None
    tuition: Optional[str] = None
    website: Optional[HttpUrl] = None
    university: Optional[WikiLink] = None
    people: list[WikiLink] = []
    links: list[LinkResource] = []
    emails: list[EmailResource] = []
