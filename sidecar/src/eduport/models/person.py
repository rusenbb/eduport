from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, LinkResource, WikiLink


class Person(BaseEntity):
    role: Optional[str] = None
    email: Optional[str] = None
    website: Optional[HttpUrl] = None
    university: Optional[WikiLink] = None
    labs: list[WikiLink] = []
    links: list[LinkResource] = []
