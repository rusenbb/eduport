from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, EmailResource, LinkResource, WikiLink


class Lab(BaseEntity):
    focus: Optional[str] = None
    website: Optional[HttpUrl] = None
    university: Optional[WikiLink] = None
    links: list[LinkResource] = []
    emails: list[EmailResource] = []
