from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, EmailResource, LinkResource


class University(BaseEntity):
    country: str
    city: Optional[str] = None
    website: Optional[HttpUrl] = None
    links: list[LinkResource] = []
    emails: list[EmailResource] = []
