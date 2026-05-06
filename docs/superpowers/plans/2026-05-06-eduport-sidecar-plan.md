# Eduport Sidecar — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Python sidecar — a FastAPI service that parses Markdown+YAML entity files, maintains a SQLite/FTS5 index with a watchdog file watcher, and exposes a REST API over HTTP loopback. End-state: a runnable backend with full CRUD, search, and `.eml` import, validated by an end-to-end smoke test.

**Architecture:** Clean-architecture-ish layering. **Models** (Pydantic) — pure data. **Parsers** — text → models. **Index** — SQLite read/write, FTS5. **Store** — file write, trash. **Watcher** — watchdog → index updates. **API** — FastAPI, depends on everything. The whole sidecar is single-process; no threads to coordinate beyond watchdog's own observer.

**Tech Stack:** Python 3.12, uv, FastAPI, uvicorn, Pydantic v2, watchdog, send2trash, platformdirs, python-frontmatter, markdown-it-py, markdownify, pytest, httpx (TestClient).

---

## File Structure

The sidecar lives at `sidecar/` in the repo root (Plan 2 will add `frontend/`, Plan 3 will add `src-tauri/`).

```
sidecar/
├── pyproject.toml                  # uv project config
├── src/eduport/
│   ├── __init__.py
│   ├── logging_setup.py            # rotating file handler in OS log dir
│   ├── settings.py                 # Settings model + TOML I/O
│   ├── slug.py                     # NFKD-fold slug generator
│   ├── ids.py                      # 4-char alphanumeric id generator
│   ├── models/
│   │   ├── __init__.py             # exports + EntityType discriminator
│   │   ├── base.py                 # BaseEntity, Resource link/email types
│   │   ├── university.py
│   │   ├── lab.py
│   │   ├── person.py
│   │   ├── program.py
│   │   ├── application.py
│   │   ├── document.py
│   │   ├── email.py
│   │   └── note.py
│   ├── parsers/
│   │   ├── __init__.py
│   │   ├── frontmatter.py          # YAML extraction
│   │   ├── wikilinks.py            # [[…]] extract + resolve
│   │   ├── checkboxes.py           # - [ ] / - [x] parsing
│   │   ├── eml.py                  # .eml → EmailParsed
│   │   └── entity.py               # dispatch by eduport-type/* tag
│   ├── index/
│   │   ├── __init__.py
│   │   ├── schema.py               # DDL incl. FTS5
│   │   ├── writer.py               # upsert/delete + parse_errors
│   │   ├── reader.py               # list/filter/search/backlinks
│   │   └── reconcile.py            # mtime sweep + full rebuild
│   ├── store/
│   │   ├── __init__.py
│   │   ├── files.py                # write/delete .md (re-entrancy guard)
│   │   └── trash.py                # send2trash + .eduport-trash fallback
│   ├── watcher.py                  # watchdog Observer + handler
│   ├── api/
│   │   ├── __init__.py
│   │   ├── app.py                  # FastAPI factory + lifespan
│   │   ├── deps.py                 # shared deps (conn, settings)
│   │   ├── health.py
│   │   ├── entities.py             # CRUD per type
│   │   ├── search.py
│   │   ├── checkbox.py
│   │   ├── eml_import.py
│   │   └── settings_api.py         # GET/PUT /settings
│   └── cli.py                      # entry: uvicorn launcher
└── tests/
    ├── __init__.py
    ├── conftest.py                 # pytest fixtures (tmp data folder, db, client)
    ├── fixtures/
    │   └── sample.eml              # used by eml parser tests
    ├── test_slug.py
    ├── test_ids.py
    ├── test_settings.py
    ├── test_models.py
    ├── test_parsers/
    │   ├── __init__.py
    │   ├── test_frontmatter.py
    │   ├── test_wikilinks.py
    │   ├── test_checkboxes.py
    │   ├── test_eml.py
    │   └── test_entity.py
    ├── test_index/
    │   ├── __init__.py
    │   ├── test_schema.py
    │   ├── test_writer.py
    │   ├── test_reader.py
    │   └── test_reconcile.py
    ├── test_store/
    │   ├── __init__.py
    │   ├── test_files.py
    │   └── test_trash.py
    ├── test_watcher.py
    └── test_api/
        ├── __init__.py
        ├── test_health.py
        ├── test_entities.py
        ├── test_search.py
        ├── test_checkbox.py
        ├── test_eml_import.py
        ├── test_settings_api.py
        └── test_e2e.py
```

**Boundaries.** Each subpackage has one job. `models/` is pure data, no I/O. `parsers/` takes text/bytes in, returns models or errors. `index/` owns SQLite. `store/` owns disk writes. `watcher.py` glues `parsers` + `store` + `index` together. `api/` is the only place FastAPI shows up.

---

## Task 0: Project scaffolding

**Files:**
- Create: `sidecar/pyproject.toml`
- Create: `sidecar/src/eduport/__init__.py`
- Create: `sidecar/tests/__init__.py`

- [ ] **Step 1: Create the sidecar project with `uv`**

Run:
```bash
cd sidecar && uv init --package --name eduport --python 3.12
```

Expected: creates `pyproject.toml`, `src/eduport/__init__.py`, `README.md`. Delete the auto-generated `README.md` (we don't want one yet — CLAUDE.md says no docs unless asked):
```bash
rm sidecar/README.md
```

- [ ] **Step 2: Add runtime + dev dependencies**

Run:
```bash
cd sidecar && uv add fastapi 'uvicorn[standard]' 'pydantic>=2' watchdog send2trash platformdirs python-frontmatter 'markdown-it-py[linkify,plugins]' markdownify
cd sidecar && uv add --dev pytest pytest-asyncio httpx ruff mypy
```

Expected: `pyproject.toml` lists all deps; `uv.lock` is created.

- [ ] **Step 3: Create the test package marker**

Create `sidecar/tests/__init__.py` (empty file).

- [ ] **Step 4: Verify the project runs `pytest`**

Run:
```bash
cd sidecar && uv run pytest -v
```

Expected: `no tests ran` exit 0 (or exit 5 — "no tests collected" — both acceptable). Confirms uv + pytest wiring is OK.

- [ ] **Step 5: Commit**

```bash
git add sidecar/
git commit -m "chore(sidecar): scaffold uv project with deps"
```

---

## Task 1: Logging setup

**Files:**
- Create: `sidecar/src/eduport/logging_setup.py`
- Test: `sidecar/tests/test_logging_setup.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_logging_setup.py`:
```python
import logging
from pathlib import Path

from eduport.logging_setup import configure_logging


def test_configure_logging_writes_to_given_path(tmp_path: Path):
    log_file = tmp_path / "eduport.log"
    configure_logging(log_file)

    logger = logging.getLogger("eduport.test")
    logger.warning("hello world")

    # Force flush all handlers
    for h in logging.getLogger("eduport").handlers:
        h.flush()

    assert log_file.exists()
    assert "hello world" in log_file.read_text()


def test_configure_logging_is_idempotent(tmp_path: Path):
    log_file = tmp_path / "eduport.log"
    configure_logging(log_file)
    configure_logging(log_file)  # second call must not double-attach handlers
    assert len(logging.getLogger("eduport").handlers) == 1
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_logging_setup.py -v
```
Expected: `ImportError: cannot import name 'configure_logging' from 'eduport.logging_setup'` (or `ModuleNotFoundError`).

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/logging_setup.py`:
```python
import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path


def configure_logging(log_file: Path, level: int = logging.INFO) -> None:
    """Attach a rotating file handler to the eduport logger.

    Idempotent: calling more than once with the same path does not duplicate handlers.
    """
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("eduport")
    logger.setLevel(level)

    if any(
        isinstance(h, RotatingFileHandler) and Path(h.baseFilename) == log_file
        for h in logger.handlers
    ):
        return

    handler = RotatingFileHandler(
        log_file, maxBytes=10 * 1024 * 1024, backupCount=3, encoding="utf-8"
    )
    handler.setFormatter(
        logging.Formatter("%(asctime)s %(levelname)s %(name)s %(message)s")
    )
    logger.addHandler(handler)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_logging_setup.py -v
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/logging_setup.py sidecar/tests/test_logging_setup.py
git commit -m "feat(sidecar): rotating file logger with idempotent setup"
```

---

## Task 2: Settings — Pydantic model + TOML I/O

**Files:**
- Create: `sidecar/src/eduport/settings.py`
- Test: `sidecar/tests/test_settings.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_settings.py`:
```python
from pathlib import Path

import pytest

from eduport.settings import Settings, load_settings, save_settings


def test_settings_round_trip(tmp_path: Path):
    settings = Settings(
        data_folder=tmp_path / "data",
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="dark",
        user_email="me@example.com",
    )
    config_file = tmp_path / "settings.toml"
    save_settings(settings, config_file)
    loaded = load_settings(config_file)
    assert loaded == settings


def test_load_missing_returns_none(tmp_path: Path):
    assert load_settings(tmp_path / "missing.toml") is None


def test_resolved_paths(tmp_path: Path):
    settings = Settings(
        data_folder=tmp_path,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )
    assert settings.resolved_attachments_folder() == tmp_path / "attachments"
    assert settings.resolved_notes_folder() == tmp_path / "notes"


def test_invalid_theme_rejected():
    with pytest.raises(ValueError):
        Settings(
            data_folder=Path("/tmp"),
            attachments_folder="./a",
            notes_folder="./n",
            theme="bogus",  # type: ignore
            user_email="x@x.com",
        )
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_settings.py -v
```
Expected: ImportError on `eduport.settings`.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/settings.py`:
```python
from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Literal, Optional

import tomli_w  # via deps? if not, `uv add tomli-w` first
from pydantic import BaseModel, ConfigDict, EmailStr, Field

Theme = Literal["system", "light", "dark"]


class Settings(BaseModel):
    model_config = ConfigDict(extra="forbid")

    data_folder: Path
    attachments_folder: str = Field(default="./attachments")
    notes_folder: str = Field(default="./notes")
    theme: Theme = "system"
    user_email: str

    def resolved_attachments_folder(self) -> Path:
        return (self.data_folder / self.attachments_folder).resolve()

    def resolved_notes_folder(self) -> Path:
        return (self.data_folder / self.notes_folder).resolve()


def load_settings(path: Path) -> Optional[Settings]:
    if not path.exists():
        return None
    return Settings.model_validate(tomllib.loads(path.read_text(encoding="utf-8")))


def save_settings(settings: Settings, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = settings.model_dump(mode="json")
    payload["data_folder"] = str(payload["data_folder"])
    path.write_bytes(tomli_w.dumps(payload).encode("utf-8"))
```

If `tomli-w` isn't already a dep:
```bash
cd sidecar && uv add tomli-w
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_settings.py -v
```
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/settings.py sidecar/tests/test_settings.py sidecar/pyproject.toml sidecar/uv.lock
git commit -m "feat(sidecar): Settings model with TOML round-trip"
```

---

## Task 3: Slug generation

**Files:**
- Create: `sidecar/src/eduport/slug.py`
- Test: `sidecar/tests/test_slug.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_slug.py`:
```python
import pytest

from eduport.slug import generate_slug


@pytest.mark.parametrize(
    "name, expected",
    [
        ("ETH Zurich", "eth-zurich"),
        ("ETH Zürich", "eth-zurich"),  # NFKD fold
        ("MSc CS (AI track)", "msc-cs-ai-track"),
        ("Søren Kierkegaard", "soren-kierkegaard"),
        ("  trailing & leading  ", "trailing-leading"),
        ("multiple --- dashes", "multiple-dashes"),
        ("CamelCase Name", "camelcase-name"),
        ("a" * 80, "a" * 60),  # truncated to 60
        ("", "untitled"),
        ("🎓🚀", "untitled"),  # emoji-only fallback
    ],
)
def test_generate_slug(name: str, expected: str):
    assert generate_slug(name) == expected


def test_truncation_at_word_boundary():
    name = "this is a fairly long sentence that exceeds sixty characters in total length"
    result = generate_slug(name)
    assert len(result) <= 60
    # No trailing partial word — must end on a complete token
    assert not result.endswith("-")
    # Token check: split and ensure last token isn't truncated mid-word
    tokens = result.split("-")
    assert all(t.isalnum() for t in tokens if t)
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_slug.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/slug.py`:
```python
import re
import unicodedata

_NON_ALNUM = re.compile(r"[^a-z0-9]+")
_MAX_LEN = 60


def generate_slug(name: str) -> str:
    # Step 1: NFKD normalize and strip combining marks
    folded = unicodedata.normalize("NFKD", name)
    folded = "".join(c for c in folded if not unicodedata.combining(c))
    # Step 2: lowercase
    folded = folded.lower()
    # Step 3: replace non-alnum runs with single hyphen
    slug = _NON_ALNUM.sub("-", folded)
    # Step 4: strip leading/trailing hyphens
    slug = slug.strip("-")
    # Step 5: truncate at word boundary (60 chars max)
    if len(slug) > _MAX_LEN:
        slug = slug[:_MAX_LEN]
        # Step 5a: if we cut mid-token, drop the partial token
        if "-" in slug:
            slug = slug.rsplit("-", 1)[0]
    # Step 6: empty fallback
    return slug or "untitled"
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_slug.py -v
```
Expected: all parametrized tests pass.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/slug.py sidecar/tests/test_slug.py
git commit -m "feat(sidecar): slug generator with NFKD fold + boundary truncation"
```

---

## Task 4: ID generation

**Files:**
- Create: `sidecar/src/eduport/ids.py`
- Test: `sidecar/tests/test_ids.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_ids.py`:
```python
import re

from eduport.ids import generate_id

_PATTERN = re.compile(r"^[a-zA-Z0-9]{4}$")


def test_id_shape():
    assert _PATTERN.match(generate_id(lambda _id: False))


def test_collision_retried():
    seen: list[str] = []

    def exists(candidate: str) -> bool:
        if len(seen) < 3:
            seen.append(candidate)
            return True  # collide three times
        return False

    result = generate_id(exists)
    assert _PATTERN.match(result)
    assert len(seen) == 3
    assert result not in seen


def test_unique_across_many_calls():
    used: set[str] = set()

    def exists(candidate: str) -> bool:
        return candidate in used

    for _ in range(200):
        new_id = generate_id(exists)
        used.add(new_id)

    assert len(used) == 200
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_ids.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/ids.py`:
```python
import secrets
from typing import Callable

_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
_LENGTH = 4
_MAX_RETRIES = 100


def generate_id(exists: Callable[[str], bool]) -> str:
    """Generate a fresh 4-char alphanumeric id. Retries on collision via `exists`."""
    for _ in range(_MAX_RETRIES):
        candidate = "".join(secrets.choice(_ALPHABET) for _ in range(_LENGTH))
        if not exists(candidate):
            return candidate
    raise RuntimeError(f"Could not generate unique id after {_MAX_RETRIES} attempts")
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_ids.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/ids.py sidecar/tests/test_ids.py
git commit -m "feat(sidecar): 4-char alphanumeric id generator with collision retry"
```

---

## Task 5: Base models — Resource types + BaseEntity + EntityType discriminator

**Files:**
- Create: `sidecar/src/eduport/models/__init__.py`
- Create: `sidecar/src/eduport/models/base.py`
- Test: `sidecar/tests/test_models.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_models.py`:
```python
import pytest
from pydantic import ValidationError

from eduport.models.base import (
    BaseEntity,
    EmailResource,
    EntityType,
    LinkResource,
    WikiLink,
)


def test_wikilink_accepts_bracketed_string():
    link = WikiLink.model_validate("[[jane-doe-A4f2]]")
    assert link.target == "jane-doe-A4f2"


def test_wikilink_rejects_unbracketed():
    with pytest.raises(ValidationError):
        WikiLink.model_validate("jane-doe-A4f2")


def test_link_resource_round_trip():
    raw = {"label": "Program page", "url": "https://example.com"}
    parsed = LinkResource.model_validate(raw)
    assert parsed.label == "Program page"
    assert str(parsed.url) == "https://example.com/"


def test_email_resource_with_optional_person():
    raw = {
        "label": "Track lead",
        "email": "jane@example.com",
        "person": "[[jane-doe-A4f2]]",
    }
    parsed = EmailResource.model_validate(raw)
    assert parsed.email == "jane@example.com"
    assert parsed.person and parsed.person.target == "jane-doe-A4f2"


def test_base_entity_required_tags_include_type():
    obj = BaseEntity.model_validate(
        {"tags": ["eduport-type/program", "ai"], "name": "Some name"}
    )
    assert obj.entity_type() == EntityType.program
    assert "ai" in obj.user_tags()


def test_base_entity_rejects_unknown_field():
    with pytest.raises(ValidationError):
        BaseEntity.model_validate(
            {"tags": ["eduport-type/program"], "name": "X", "bogus_field": 1}
        )
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/models/base.py`:
```python
from __future__ import annotations

import re
from enum import Enum
from typing import Optional

from pydantic import BaseModel, ConfigDict, Field, HttpUrl, field_validator


class EntityType(str, Enum):
    university = "university"
    lab = "lab"
    person = "person"
    program = "program"
    application = "application"
    document = "document"
    email = "email"
    note = "note"


_WIKILINK_RE = re.compile(r"^\[\[([^\]\[]+)\]\]$")


class WikiLink(BaseModel):
    """A `[[target]]` reference. `target` is the filename stem (no .md, no brackets)."""

    model_config = ConfigDict(frozen=True)
    target: str

    @classmethod
    def model_validate(cls, value, *args, **kwargs):  # type: ignore[override]
        if isinstance(value, str):
            m = _WIKILINK_RE.match(value)
            if not m:
                raise ValueError(f"Not a wikilink: {value!r}")
            return cls(target=m.group(1).strip())
        return super().model_validate(value, *args, **kwargs)

    def __str__(self) -> str:
        return f"[[{self.target}]]"


class LinkResource(BaseModel):
    model_config = ConfigDict(extra="forbid")
    label: str
    url: HttpUrl


class EmailResource(BaseModel):
    model_config = ConfigDict(extra="forbid")
    label: str
    email: str
    person: Optional[WikiLink] = None


_TYPE_PREFIX = "eduport-type/"


class BaseEntity(BaseModel):
    model_config = ConfigDict(extra="forbid")

    tags: list[str] = Field(default_factory=list)
    name: str

    def entity_type(self) -> EntityType:
        for tag in self.tags:
            if tag.startswith(_TYPE_PREFIX):
                return EntityType(tag[len(_TYPE_PREFIX):])
        raise ValueError("missing eduport-type/* tag")

    def user_tags(self) -> list[str]:
        return [
            t for t in self.tags
            if not t.startswith(_TYPE_PREFIX) and not t.startswith("eduport-doctype/")
        ]

    @field_validator("tags")
    @classmethod
    def _has_type_tag(cls, tags: list[str]) -> list[str]:
        if not any(t.startswith(_TYPE_PREFIX) for t in tags):
            raise ValueError("entity must have an eduport-type/* tag")
        return tags
```

`sidecar/src/eduport/models/__init__.py`:
```python
from eduport.models.base import (
    BaseEntity,
    EmailResource,
    EntityType,
    LinkResource,
    WikiLink,
)

__all__ = [
    "BaseEntity",
    "EmailResource",
    "EntityType",
    "LinkResource",
    "WikiLink",
]
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/models/ sidecar/tests/test_models.py
git commit -m "feat(sidecar): base models — WikiLink, Resource types, BaseEntity"
```

---

## Task 6: Entity models — University, Lab, Person

**Files:**
- Create: `sidecar/src/eduport/models/university.py`
- Create: `sidecar/src/eduport/models/lab.py`
- Create: `sidecar/src/eduport/models/person.py`
- Modify: `sidecar/src/eduport/models/__init__.py`
- Test: append to `sidecar/tests/test_models.py`

- [ ] **Step 1: Write the failing tests (append)**

Append to `sidecar/tests/test_models.py`:
```python
from eduport.models import Lab, Person, University


def test_university_minimal():
    u = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH Zurich",
        "country": "Switzerland",
    })
    assert u.country == "Switzerland"
    assert u.links == []
    assert u.emails == []


def test_university_with_resources():
    u = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH",
        "country": "CH",
        "links": [{"label": "Admissions", "url": "https://ethz.ch/apply"}],
        "emails": [{"label": "Info", "email": "info@ethz.ch"}],
    })
    assert len(u.links) == 1
    assert u.emails[0].email == "info@ethz.ch"


def test_lab_with_university_link():
    lab = Lab.model_validate({
        "tags": ["eduport-type/lab"],
        "name": "MLG",
        "university": "[[eth-zurich-K9p3]]",
    })
    assert lab.university and lab.university.target == "eth-zurich-K9p3"


def test_person_full():
    p = Person.model_validate({
        "tags": ["eduport-type/person", "ai"],
        "name": "Jane Doe",
        "role": "Professor",
        "email": "jane@example.com",
        "university": "[[eth-zurich-K9p3]]",
        "labs": ["[[mlg-B2n4]]"],
    })
    assert p.role == "Professor"
    assert len(p.labs) == 1
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: ImportError on `Lab`/`Person`/`University`.

- [ ] **Step 3: Write minimal implementations**

`sidecar/src/eduport/models/university.py`:
```python
from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, EmailResource, LinkResource


class University(BaseEntity):
    country: str
    city: Optional[str] = None
    website: Optional[HttpUrl] = None
    links: list[LinkResource] = []
    emails: list[EmailResource] = []
```

`sidecar/src/eduport/models/lab.py`:
```python
from typing import Optional

from pydantic import HttpUrl

from eduport.models.base import BaseEntity, EmailResource, LinkResource, WikiLink


class Lab(BaseEntity):
    focus: Optional[str] = None
    website: Optional[HttpUrl] = None
    university: Optional[WikiLink] = None
    links: list[LinkResource] = []
    emails: list[EmailResource] = []
```

`sidecar/src/eduport/models/person.py`:
```python
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
```

Update `sidecar/src/eduport/models/__init__.py`:
```python
from eduport.models.base import (
    BaseEntity,
    EmailResource,
    EntityType,
    LinkResource,
    WikiLink,
)
from eduport.models.lab import Lab
from eduport.models.person import Person
from eduport.models.university import University

__all__ = [
    "BaseEntity",
    "EmailResource",
    "EntityType",
    "Lab",
    "LinkResource",
    "Person",
    "University",
    "WikiLink",
]
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: 10 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/models/ sidecar/tests/test_models.py
git commit -m "feat(sidecar): University, Lab, Person models"
```

---

## Task 7: Entity models — Program, Application

**Files:**
- Create: `sidecar/src/eduport/models/program.py`
- Create: `sidecar/src/eduport/models/application.py`
- Modify: `sidecar/src/eduport/models/__init__.py`
- Test: append to `sidecar/tests/test_models.py`

- [ ] **Step 1: Write the failing tests (append)**

Append to `sidecar/tests/test_models.py`:
```python
from datetime import date

from eduport.models import Application, ApplicationStatus, Level, Program


def test_program_full():
    p = Program.model_validate({
        "tags": ["eduport-type/program", "ai"],
        "name": "MSc CS",
        "level": "masters",
        "deadline": "2026-12-15",
        "university": "[[eth-zurich-K9p3]]",
        "people": ["[[jane-doe-A4f2]]"],
        "links": [{"label": "Page", "url": "https://x.example"}],
    })
    assert p.level == Level.masters
    assert p.deadline == date(2026, 12, 15)
    assert p.people[0].target == "jane-doe-A4f2"


def test_program_invalid_level_rejected():
    with pytest.raises(ValidationError):
        Program.model_validate({
            "tags": ["eduport-type/program"],
            "name": "X",
            "level": "bogus",
        })


def test_application_minimal():
    a = Application.model_validate({
        "tags": ["eduport-type/application"],
        "name": "ETH 2026",
        "program": "[[msc-cs-Q7w8]]",
        "status": "drafting",
    })
    assert a.status == ApplicationStatus.drafting
    assert a.documents == []
    assert a.submitted_at is None
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementations**

`sidecar/src/eduport/models/program.py`:
```python
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
```

`sidecar/src/eduport/models/application.py`:
```python
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
```

Append to `sidecar/src/eduport/models/__init__.py` exports:
```python
from eduport.models.application import Application, ApplicationStatus
from eduport.models.program import Level, Program
```

Add to `__all__`: `"Application"`, `"ApplicationStatus"`, `"Level"`, `"Program"`.

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: 13 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/models/ sidecar/tests/test_models.py
git commit -m "feat(sidecar): Program and Application models"
```

---

## Task 8: Entity models — Document, Email, Note

**Files:**
- Create: `sidecar/src/eduport/models/document.py`
- Create: `sidecar/src/eduport/models/email.py`
- Create: `sidecar/src/eduport/models/note.py`
- Modify: `sidecar/src/eduport/models/__init__.py`
- Test: append to `sidecar/tests/test_models.py`

- [ ] **Step 1: Write the failing tests (append)**

Append to `sidecar/tests/test_models.py`:
```python
from eduport.models import (
    Document,
    DocumentStatus,
    Email,
    EmailDirection,
    Note,
)


def test_document_received_default():
    d = Document.model_validate({
        "tags": ["eduport-type/document", "eduport-doctype/cv"],
        "name": "CV March 2026",
        "title": "CV",
        "file": "attachments/cv.pdf",
    })
    assert d.status == DocumentStatus.received  # default when file present


def test_document_pending_recommendation():
    d = Document.model_validate({
        "tags": ["eduport-type/document", "eduport-doctype/recommendation"],
        "name": "Rec letter",
        "title": "Rec from Jane",
        "status": "requested",
        "recommender": "[[jane-doe-A4f2]]",
        "requested_at": "2026-10-01",
    })
    assert d.status == DocumentStatus.requested
    assert d.file is None
    assert d.recommender and d.recommender.target == "jane-doe-A4f2"


def test_email_full():
    e = Email.model_validate({
        "tags": ["eduport-type/email"],
        "name": "Q about deadline",
        "direction": "outbound",
        "date": "2026-09-20",
        "subject": "Question about MSc CS deadline",
        "from": "rusen@example.com",
        "to": ["admissions@inf.ethz.ch"],
        "cc": ["jane.doe@inf.ethz.ch"],
        "related_program": "[[msc-cs-Q7w8]]",
        "related_people": ["[[jane-doe-A4f2]]"],
    })
    assert e.direction == EmailDirection.outbound
    assert e.from_ == "rusen@example.com"
    assert e.cc == ["jane.doe@inf.ethz.ch"]
    assert e.related_program and e.related_program.target == "msc-cs-Q7w8"


def test_note_minimal():
    n = Note.model_validate({
        "tags": ["eduport-type/note"],
        "name": "scratchpad",
    })
    assert n.entity_type().value == "note"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementations**

`sidecar/src/eduport/models/document.py`:
```python
from datetime import date
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
    date: Optional[date] = None
    file: Optional[str] = None  # path relative to data folder
    status: Optional[DocumentStatus] = None
    requested_at: Optional[date] = None
    recommender: Optional[WikiLink] = None

    @model_validator(mode="after")
    def _default_status(self) -> "Document":
        if self.status is None:
            self.status = DocumentStatus.received if self.file else DocumentStatus.drafting
        return self
```

`sidecar/src/eduport/models/email.py`:
```python
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
```

`sidecar/src/eduport/models/note.py`:
```python
from eduport.models.base import BaseEntity


class Note(BaseEntity):
    pass
```

Append to `sidecar/src/eduport/models/__init__.py`:
```python
from eduport.models.document import Document, DocumentStatus
from eduport.models.email import Email, EmailDirection
from eduport.models.note import Note
```
Add to `__all__`: `"Document"`, `"DocumentStatus"`, `"Email"`, `"EmailDirection"`, `"Note"`.

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_models.py -v
```
Expected: 17 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/models/ sidecar/tests/test_models.py
git commit -m "feat(sidecar): Document, Email, Note models"
```

---

## Task 9: Frontmatter parser

**Files:**
- Create: `sidecar/src/eduport/parsers/__init__.py`
- Create: `sidecar/src/eduport/parsers/frontmatter.py`
- Test: `sidecar/tests/test_parsers/__init__.py` and `tests/test_parsers/test_frontmatter.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_parsers/__init__.py`: empty file.

`sidecar/tests/test_parsers/test_frontmatter.py`:
```python
import pytest

from eduport.parsers.frontmatter import FrontmatterError, split


def test_split_basic():
    raw = """---
name: ETH
tags: [eduport-type/university]
---

Body text here.
"""
    fm, body = split(raw)
    assert fm == {"name": "ETH", "tags": ["eduport-type/university"]}
    assert body.strip() == "Body text here."


def test_split_no_frontmatter():
    raw = "Just a body, no frontmatter."
    fm, body = split(raw)
    assert fm == {}
    assert body == raw


def test_split_invalid_yaml_raises():
    raw = """---
name: ETH
tags: [unclosed
---

Body
"""
    with pytest.raises(FrontmatterError):
        split(raw)


def test_empty_frontmatter():
    raw = """---
---

Body
"""
    fm, body = split(raw)
    assert fm == {}
    assert body.strip() == "Body"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_frontmatter.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/parsers/__init__.py`: empty file.

`sidecar/src/eduport/parsers/frontmatter.py`:
```python
from typing import Any

import yaml


class FrontmatterError(ValueError):
    pass


def split(raw: str) -> tuple[dict[str, Any], str]:
    """Return (frontmatter_dict, body_str). Empty dict if no frontmatter."""
    if not raw.startswith("---"):
        return {}, raw
    # Skip leading ---\n
    after_open = raw[3:]
    end = after_open.find("\n---")
    if end == -1:
        return {}, raw
    yaml_block = after_open[:end].strip()
    # Body starts after \n--- and the trailing newline
    body_start = end + len("\n---")
    # Skip exactly one trailing newline if present
    if body_start < len(after_open) and after_open[body_start] == "\n":
        body_start += 1
    body = after_open[body_start:]

    if not yaml_block:
        return {}, body

    try:
        parsed = yaml.safe_load(yaml_block)
    except yaml.YAMLError as exc:
        raise FrontmatterError(str(exc)) from exc

    if parsed is None:
        return {}, body
    if not isinstance(parsed, dict):
        raise FrontmatterError(f"Frontmatter must be a mapping, got {type(parsed).__name__}")
    return parsed, body
```

If `pyyaml` isn't already a dep:
```bash
cd sidecar && uv add pyyaml
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_frontmatter.py -v
```
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/parsers/ sidecar/tests/test_parsers/ sidecar/pyproject.toml sidecar/uv.lock
git commit -m "feat(sidecar): YAML frontmatter splitter with error reporting"
```

---

## Task 10: Wikilink extraction + resolution

**Files:**
- Create: `sidecar/src/eduport/parsers/wikilinks.py`
- Test: `sidecar/tests/test_parsers/test_wikilinks.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_parsers/test_wikilinks.py`:
```python
from eduport.parsers.wikilinks import extract_targets, resolve


def test_extract_from_string_value():
    assert extract_targets("[[jane-A4f2]]") == ["jane-A4f2"]
    assert extract_targets("plain string") == []


def test_extract_from_nested_structure():
    payload = {
        "university": "[[eth-K9p3]]",
        "people": ["[[a-1111]]", "[[b-2222]]"],
        "emails": [{"label": "x", "email": "y", "person": "[[c-3333]]"}],
    }
    found = sorted(extract_targets(payload))
    assert found == ["a-1111", "b-2222", "c-3333", "eth-K9p3"]


def test_resolve_exact_match():
    candidates = ["jane-doe-A4f2", "bob-K9p3", "msc-cs-Q7w8"]
    assert resolve("jane-doe-A4f2", candidates) == "jane-doe-A4f2"


def test_resolve_id_suffix_fallback():
    candidates = ["jane-doe-renamed-A4f2", "bob-K9p3"]
    assert resolve("jane-doe-A4f2", candidates) == "jane-doe-renamed-A4f2"


def test_resolve_broken_returns_none():
    assert resolve("ghost-Z9z9", ["jane-A4f2"]) is None


def test_resolve_ambiguous_id_picks_none():
    # Two candidates share the same id suffix — that's a vault corruption
    candidates = ["a-A4f2", "b-A4f2"]
    assert resolve("c-A4f2", candidates) is None
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_wikilinks.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/parsers/wikilinks.py`:
```python
import re
from typing import Any, Optional, Sequence

_WIKILINK_RE = re.compile(r"\[\[([^\]\[]+)\]\]")
_ID_SUFFIX_RE = re.compile(r"-([a-zA-Z0-9]{4})$")


def extract_targets(value: Any) -> list[str]:
    """Walk a JSON-ish value and collect all wikilink targets."""
    found: list[str] = []
    _walk(value, found)
    return found


def _walk(value: Any, into: list[str]) -> None:
    if isinstance(value, str):
        for m in _WIKILINK_RE.finditer(value):
            into.append(m.group(1).strip())
    elif isinstance(value, dict):
        for v in value.values():
            _walk(v, into)
    elif isinstance(value, (list, tuple)):
        for v in value:
            _walk(v, into)


def resolve(target: str, candidates: Sequence[str]) -> Optional[str]:
    """Resolve a wikilink target against existing file stems.

    1. Exact match wins.
    2. Otherwise find a unique candidate sharing the same trailing 4-char id.
    3. Returns None if no resolution.
    """
    if target in candidates:
        return target
    m = _ID_SUFFIX_RE.search(target)
    if not m:
        return None
    id_part = m.group(1)
    matches = [c for c in candidates if c.endswith(f"-{id_part}")]
    if len(matches) == 1:
        return matches[0]
    return None
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_wikilinks.py -v
```
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/parsers/wikilinks.py sidecar/tests/test_parsers/test_wikilinks.py
git commit -m "feat(sidecar): wikilink extract + resolve with id-suffix fallback"
```

---

## Task 11: Checkbox parser

**Files:**
- Create: `sidecar/src/eduport/parsers/checkboxes.py`
- Test: `sidecar/tests/test_parsers/test_checkboxes.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_parsers/test_checkboxes.py`:
```python
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
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_checkboxes.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/parsers/checkboxes.py`:
```python
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
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_checkboxes.py -v
```
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/parsers/checkboxes.py sidecar/tests/test_parsers/test_checkboxes.py
git commit -m "feat(sidecar): inline checkbox parser with optional ISO date"
```

---

## Task 12: Entity dispatcher (parse + classify)

**Files:**
- Create: `sidecar/src/eduport/parsers/entity.py`
- Test: `sidecar/tests/test_parsers/test_entity.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_parsers/test_entity.py`:
```python
from pathlib import Path

import pytest

from eduport.models import EntityType, Program, University
from eduport.parsers.entity import ParseError, parse_file


def _write(tmp_path: Path, name: str, content: str) -> Path:
    p = tmp_path / name
    p.write_text(content, encoding="utf-8")
    return p


def test_parse_university(tmp_path: Path):
    raw = """---
tags: [eduport-type/university]
name: ETH
country: Switzerland
---

Body
"""
    path = _write(tmp_path, "eth-K9p3.md", raw)
    result = parse_file(path)
    assert isinstance(result, University)
    assert result.name == "ETH"


def test_parse_program(tmp_path: Path):
    raw = """---
tags: [eduport-type/program]
name: MSc CS
level: masters
---
"""
    path = _write(tmp_path, "msc-Q7w8.md", raw)
    result = parse_file(path)
    assert isinstance(result, Program)


def test_parse_returns_error_on_invalid_yaml(tmp_path: Path):
    raw = """---
tags: [unclosed
---
"""
    path = _write(tmp_path, "bad-X1x1.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)
    assert "yaml" in result.message.lower() or "frontmatter" in result.message.lower()


def test_parse_returns_error_on_missing_type_tag(tmp_path: Path):
    raw = """---
tags: [random]
name: X
---
"""
    path = _write(tmp_path, "x-Y2y2.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)


def test_parse_unknown_type_tag(tmp_path: Path):
    raw = """---
tags: [eduport-type/imaginary]
name: X
---
"""
    path = _write(tmp_path, "x-Z3z3.md", raw)
    result = parse_file(path)
    assert isinstance(result, ParseError)
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_entity.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/parsers/entity.py`:
```python
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Union

from pydantic import ValidationError

from eduport.models import (
    Application,
    BaseEntity,
    Document,
    Email,
    EntityType,
    Lab,
    Note,
    Person,
    Program,
    University,
)
from eduport.parsers.frontmatter import FrontmatterError, split

_TYPE_TO_MODEL: dict[EntityType, type[BaseEntity]] = {
    EntityType.university: University,
    EntityType.lab: Lab,
    EntityType.person: Person,
    EntityType.program: Program,
    EntityType.application: Application,
    EntityType.document: Document,
    EntityType.email: Email,
    EntityType.note: Note,
}


@dataclass(frozen=True)
class ParseError:
    path: Path
    message: str


@dataclass(frozen=True)
class ParsedEntity:
    """Wraps a typed model + raw body + path."""
    entity: BaseEntity
    body: str
    path: Path


ParseResult = Union[ParsedEntity, ParseError]


def parse_file(path: Path) -> ParseResult:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as exc:
        return ParseError(path=path, message=f"read failed: {exc}")

    try:
        fm, body = split(raw)
    except FrontmatterError as exc:
        return ParseError(path=path, message=f"frontmatter error: {exc}")

    if not fm:
        return ParseError(path=path, message="missing frontmatter")

    # Inject `name` if absent — fall back to filename stem
    fm.setdefault("name", path.stem)

    type_tag = next(
        (t for t in fm.get("tags", []) if t.startswith("eduport-type/")),
        None,
    )
    if type_tag is None:
        return ParseError(path=path, message="missing eduport-type/* tag")

    type_value = type_tag.removeprefix("eduport-type/")
    try:
        entity_type = EntityType(type_value)
    except ValueError:
        return ParseError(path=path, message=f"unknown entity type: {type_value!r}")

    model_cls = _TYPE_TO_MODEL[entity_type]
    try:
        entity = model_cls.model_validate(fm)
    except ValidationError as exc:
        return ParseError(path=path, message=f"validation error: {exc}")

    return ParsedEntity(entity=entity, body=body, path=path)
```

Update test to import `parse_file` and assert against `ParsedEntity` instead of bare model. Adjust test:
```python
# in each successful test, change:
result = parse_file(path)
assert isinstance(result, ParsedEntity)
assert isinstance(result.entity, University)
```
(Apply the same `ParsedEntity` wrapping to `test_parse_university`, `test_parse_program`. Use `from eduport.parsers.entity import ParsedEntity, ParseError, parse_file`.)

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_entity.py -v
```
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/parsers/entity.py sidecar/tests/test_parsers/test_entity.py
git commit -m "feat(sidecar): entity parser dispatching by eduport-type tag"
```

---

## Task 13: `.eml` parser

**Files:**
- Create: `sidecar/tests/fixtures/sample.eml`
- Create: `sidecar/src/eduport/parsers/eml.py`
- Test: `sidecar/tests/test_parsers/test_eml.py`

- [ ] **Step 1: Write the failing test + create the fixture**

`sidecar/tests/fixtures/sample.eml`:
```
From: Jane Doe <jane@example.com>
To: rusen@example.com
Cc: bob@example.com
Subject: Welcome to ETH
Date: Fri, 20 Sep 2026 10:00:00 +0000
Content-Type: text/plain; charset=utf-8

Hi Rusen,

Welcome to the program! Looking forward to meeting you.

Best,
Jane
```

`sidecar/tests/test_parsers/test_eml.py`:
```python
from datetime import date
from pathlib import Path

from eduport.models import EmailDirection
from eduport.parsers.eml import parse_eml


def test_parse_sample_eml():
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    parsed = parse_eml(fixture.read_bytes(), user_email="rusen@example.com")
    assert parsed.from_ == "jane@example.com"
    assert parsed.to == ["rusen@example.com"]
    assert parsed.cc == ["bob@example.com"]
    assert parsed.subject == "Welcome to ETH"
    assert parsed.date == date(2026, 9, 20)
    assert parsed.direction == EmailDirection.inbound
    assert "Welcome to the program" in parsed.body


def test_outbound_inferred_when_user_in_from():
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    parsed = parse_eml(fixture.read_bytes(), user_email="jane@example.com")
    assert parsed.direction == EmailDirection.outbound
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_eml.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/parsers/eml.py`:
```python
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
    # Prefer text/plain. Fall back to converted HTML.
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
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_parsers/test_eml.py -v
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/parsers/eml.py sidecar/tests/test_parsers/test_eml.py sidecar/tests/fixtures/
git commit -m "feat(sidecar): .eml parser with direction inference"
```

---

## Task 14: SQLite schema

**Files:**
- Create: `sidecar/src/eduport/index/__init__.py`
- Create: `sidecar/src/eduport/index/schema.py`
- Test: `sidecar/tests/test_index/__init__.py` and `test_schema.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_index/__init__.py`: empty.

`sidecar/tests/test_index/test_schema.py`:
```python
import sqlite3

from eduport.index.schema import init_schema


def test_creates_all_tables(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)

    tables = {row[0] for row in conn.execute(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view')"
    )}
    expected = {
        "entities", "entity_tags", "entity_links",
        "checkboxes", "parse_errors", "entities_fts",
    }
    assert expected <= tables


def test_idempotent(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)
    init_schema(conn)  # second call must not error


def test_fts_supports_match(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)
    conn.execute(
        "INSERT INTO entities_fts(rowid, body) VALUES (1, 'the quick brown fox')"
    )
    rows = conn.execute(
        "SELECT rowid FROM entities_fts WHERE entities_fts MATCH 'fox'"
    ).fetchall()
    assert rows == [(1,)]
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_index/test_schema.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/index/__init__.py`: empty.

`sidecar/src/eduport/index/schema.py`:
```python
import sqlite3

_DDL = """
CREATE TABLE IF NOT EXISTS entities (
    file_id     TEXT PRIMARY KEY,            -- filename stem (no .md), e.g. "eth-zurich-K9p3"
    type        TEXT NOT NULL,               -- 'university' | 'lab' | ...
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,               -- absolute path to .md
    mtime_ns    INTEGER NOT NULL,
    body        TEXT NOT NULL,
    frontmatter TEXT NOT NULL                -- raw JSON of validated model
);

CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);

CREATE TABLE IF NOT EXISTS entity_tags (
    file_id TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (file_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_entity_tags_tag ON entity_tags(tag);

CREATE TABLE IF NOT EXISTS entity_links (
    src_file_id TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    field       TEXT NOT NULL,                -- e.g. 'university', 'people', 'documents'
    target      TEXT NOT NULL,                -- raw target (may be unresolved)
    resolved    TEXT,                         -- resolved file_id or NULL if broken
    PRIMARY KEY (src_file_id, field, target)
);
CREATE INDEX IF NOT EXISTS idx_links_resolved ON entity_links(resolved);

CREATE TABLE IF NOT EXISTS checkboxes (
    file_id  TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    line     INTEGER NOT NULL,
    checked  INTEGER NOT NULL,
    text     TEXT NOT NULL,
    deadline TEXT,                              -- ISO date, nullable
    PRIMARY KEY (file_id, line)
);

CREATE TABLE IF NOT EXISTS parse_errors (
    path        TEXT PRIMARY KEY,
    message     TEXT NOT NULL,
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
    body,
    name,
    tags,
    tokenize="unicode61 remove_diacritics 2"
);
"""


def init_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(_DDL)
    conn.commit()
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_index/test_schema.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/index/ sidecar/tests/test_index/
git commit -m "feat(sidecar): SQLite schema with FTS5 virtual table"
```

---

## Task 15: Index writer (upsert + delete)

**Files:**
- Create: `sidecar/src/eduport/index/writer.py`
- Test: `sidecar/tests/test_index/test_writer.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_index/test_writer.py`:
```python
import sqlite3
from pathlib import Path

from eduport.index.schema import init_schema
from eduport.index.writer import delete_entity, upsert_entity
from eduport.models import University


def _conn(tmp_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(tmp_path / "x.db")
    init_schema(conn)
    return conn


def _make_uni() -> University:
    return University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH",
        "country": "Switzerland",
    })


def test_upsert_inserts(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(
        conn,
        file_id="eth-K9p3",
        path=Path("/data/eth-K9p3.md"),
        mtime_ns=12345,
        entity=_make_uni(),
        body="Body",
    )
    rows = conn.execute("SELECT file_id, type, name FROM entities").fetchall()
    assert rows == [("eth-K9p3", "university", "ETH")]

    tags = {row[0] for row in conn.execute(
        "SELECT tag FROM entity_tags WHERE file_id='eth-K9p3'"
    )}
    assert tags == {"eduport-type/university", "switzerland"}

    fts = conn.execute(
        "SELECT body FROM entities_fts WHERE rowid = (SELECT rowid FROM entities WHERE file_id='eth-K9p3')"
    ).fetchone()
    assert fts == ("Body",)


def test_upsert_updates_on_conflict(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, _make_uni(), "old body")
    new_uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH (renamed)",
        "country": "CH",
    })
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 2, new_uni, "new body")

    name = conn.execute("SELECT name FROM entities").fetchone()[0]
    assert name == "ETH (renamed)"
    body = conn.execute(
        "SELECT body FROM entities_fts WHERE rowid = (SELECT rowid FROM entities WHERE file_id='eth-K9p3')"
    ).fetchone()[0]
    assert body == "new body"
    # tags must reflect the new state (no orphan rows)
    tags = {row[0] for row in conn.execute("SELECT tag FROM entity_tags")}
    assert tags == {"eduport-type/university"}


def test_delete_clears_everything(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, _make_uni(), "body")
    delete_entity(conn, "eth-K9p3")
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM entity_tags").fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM entities_fts").fetchone()[0] == 0
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_index/test_writer.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/index/writer.py`:
```python
from __future__ import annotations

import json
import sqlite3
from pathlib import Path

from eduport.models import BaseEntity
from eduport.parsers.wikilinks import extract_targets


def upsert_entity(
    conn: sqlite3.Connection,
    file_id: str,
    path: Path,
    mtime_ns: int,
    entity: BaseEntity,
    body: str,
) -> None:
    fm_json = entity.model_dump_json(by_alias=True)
    cur = conn.cursor()
    cur.execute("BEGIN")
    try:
        cur.execute(
            "INSERT OR REPLACE INTO entities(file_id, type, name, path, mtime_ns, body, frontmatter) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                file_id,
                entity.entity_type().value,
                entity.name,
                str(path),
                mtime_ns,
                body,
                fm_json,
            ),
        )
        rowid = cur.execute(
            "SELECT rowid FROM entities WHERE file_id = ?", (file_id,)
        ).fetchone()[0]

        # Replace tags
        cur.execute("DELETE FROM entity_tags WHERE file_id = ?", (file_id,))
        cur.executemany(
            "INSERT INTO entity_tags(file_id, tag) VALUES (?, ?)",
            [(file_id, t) for t in entity.tags],
        )

        # Replace links
        cur.execute("DELETE FROM entity_links WHERE src_file_id = ?", (file_id,))
        fm_payload = json.loads(fm_json)
        link_rows = []
        for field, value in fm_payload.items():
            for target in extract_targets(value):
                link_rows.append((file_id, field, target, None))
        if link_rows:
            cur.executemany(
                "INSERT OR IGNORE INTO entity_links(src_file_id, field, target, resolved) "
                "VALUES (?, ?, ?, ?)",
                link_rows,
            )

        # Replace FTS row
        cur.execute("DELETE FROM entities_fts WHERE rowid = ?", (rowid,))
        cur.execute(
            "INSERT INTO entities_fts(rowid, body, name, tags) VALUES (?, ?, ?, ?)",
            (rowid, body, entity.name, " ".join(entity.tags)),
        )
        conn.commit()
    except Exception:
        conn.rollback()
        raise


def delete_entity(conn: sqlite3.Connection, file_id: str) -> None:
    cur = conn.cursor()
    cur.execute("BEGIN")
    try:
        rowid_row = cur.execute(
            "SELECT rowid FROM entities WHERE file_id = ?", (file_id,)
        ).fetchone()
        if rowid_row is not None:
            cur.execute("DELETE FROM entities_fts WHERE rowid = ?", (rowid_row[0],))
        cur.execute("DELETE FROM entities WHERE file_id = ?", (file_id,))
        # entity_tags / entity_links cascade via foreign keys, but enable enforcement:
        cur.execute("DELETE FROM entity_tags WHERE file_id = ?", (file_id,))
        cur.execute("DELETE FROM entity_links WHERE src_file_id = ?", (file_id,))
        conn.commit()
    except Exception:
        conn.rollback()
        raise
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_index/test_writer.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/index/writer.py sidecar/tests/test_index/test_writer.py
git commit -m "feat(sidecar): index writer (upsert/delete) with FTS sync"
```

---

## Task 16: Parse-error tracking

**Files:**
- Modify: `sidecar/src/eduport/index/writer.py` (append two functions)
- Test: append to `sidecar/tests/test_index/test_writer.py`

- [ ] **Step 1: Write the failing test (append)**

Append to `sidecar/tests/test_index/test_writer.py`:
```python
from eduport.index.writer import clear_parse_error, record_parse_error


def test_record_and_clear_parse_error(tmp_path):
    conn = _conn(tmp_path)
    record_parse_error(conn, "/data/bad-X1x1.md", "bad yaml")
    rows = conn.execute("SELECT path, message FROM parse_errors").fetchall()
    assert rows == [("/data/bad-X1x1.md", "bad yaml")]

    clear_parse_error(conn, "/data/bad-X1x1.md")
    assert conn.execute("SELECT COUNT(*) FROM parse_errors").fetchone()[0] == 0


def test_record_overwrites(tmp_path):
    conn = _conn(tmp_path)
    record_parse_error(conn, "/data/bad-X.md", "first")
    record_parse_error(conn, "/data/bad-X.md", "second")
    rows = conn.execute("SELECT message FROM parse_errors").fetchall()
    assert rows == [("second",)]
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_index/test_writer.py -v
```
Expected: ImportError on `record_parse_error`.

- [ ] **Step 3: Append minimal implementation to `sidecar/src/eduport/index/writer.py`:**

```python
def record_parse_error(conn: sqlite3.Connection, path: str, message: str) -> None:
    conn.execute(
        "INSERT OR REPLACE INTO parse_errors(path, message) VALUES (?, ?)",
        (path, message),
    )
    conn.commit()


def clear_parse_error(conn: sqlite3.Connection, path: str) -> None:
    conn.execute("DELETE FROM parse_errors WHERE path = ?", (path,))
    conn.commit()
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_index/test_writer.py -v
```
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/index/writer.py sidecar/tests/test_index/test_writer.py
git commit -m "feat(sidecar): parse-error recording in index writer"
```

---

## Task 17: Index reader — list, filter, backlinks

**Files:**
- Create: `sidecar/src/eduport/index/reader.py`
- Test: `sidecar/tests/test_index/test_reader.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_index/test_reader.py`:
```python
import sqlite3
from pathlib import Path

import pytest

from eduport.index.reader import backlinks, list_entities, search_fts
from eduport.index.schema import init_schema
from eduport.index.writer import upsert_entity
from eduport.models import Person, Program, University


@pytest.fixture
def conn(tmp_path) -> sqlite3.Connection:
    c = sqlite3.connect(tmp_path / "x.db")
    init_schema(c)

    eth = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH", "country": "Switzerland",
    })
    msc = Program.model_validate({
        "tags": ["eduport-type/program", "ai"],
        "name": "MSc CS", "level": "masters",
        "university": "[[eth-K9p3]]",
        "people": ["[[jane-A4f2]]"],
    })
    jane = Person.model_validate({
        "tags": ["eduport-type/person", "ai"],
        "name": "Jane Doe", "role": "Professor",
    })
    upsert_entity(c, "eth-K9p3", Path("/x/eth.md"), 1, eth, "Body of ETH")
    upsert_entity(c, "msc-Q7w8", Path("/x/msc.md"), 1, msc, "Body of MSc CS")
    upsert_entity(c, "jane-A4f2", Path("/x/jane.md"), 1, jane, "Body about machine learning")
    return c


def test_list_by_type(conn):
    universities = list_entities(conn, type="university")
    assert [r["file_id"] for r in universities] == ["eth-K9p3"]


def test_list_filter_by_tag(conn):
    ai_only = list_entities(conn, tags=["ai"])
    ids = sorted(r["file_id"] for r in ai_only)
    assert ids == ["jane-A4f2", "msc-Q7w8"]


def test_list_filter_by_multiple_tags_and(conn):
    rows = list_entities(conn, tags=["ai", "switzerland"])
    assert rows == []  # AND semantics — no entity has both


def test_search_body(conn):
    hits = search_fts(conn, "machine learning")
    assert [h["file_id"] for h in hits] == ["jane-A4f2"]


def test_backlinks(conn):
    # MSc has links to eth-K9p3 and jane-A4f2; resolve them
    conn.execute(
        "UPDATE entity_links SET resolved = target WHERE src_file_id = 'msc-Q7w8'"
    )
    incoming = backlinks(conn, "jane-A4f2")
    assert [b["src_file_id"] for b in incoming] == ["msc-Q7w8"]
    assert incoming[0]["field"] == "people"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_index/test_reader.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/index/reader.py`:
```python
from __future__ import annotations

import sqlite3
from typing import Optional, Sequence


def _row_to_dict(row: sqlite3.Row) -> dict:
    return {k: row[k] for k in row.keys()}


def list_entities(
    conn: sqlite3.Connection,
    type: Optional[str] = None,
    tags: Optional[Sequence[str]] = None,
) -> list[dict]:
    conn.row_factory = sqlite3.Row
    where: list[str] = []
    params: list[object] = []
    if type is not None:
        where.append("type = ?")
        params.append(type)
    if tags:
        # AND semantics: entity must have every tag.
        placeholders = ", ".join("?" * len(tags))
        where.append(
            f"file_id IN ("
            f"  SELECT file_id FROM entity_tags "
            f"  WHERE tag IN ({placeholders}) "
            f"  GROUP BY file_id HAVING COUNT(DISTINCT tag) = ?"
            f")"
        )
        params.extend(tags)
        params.append(len(tags))
    sql = "SELECT file_id, type, name, path, mtime_ns FROM entities"
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY name"
    return [_row_to_dict(row) for row in conn.execute(sql, params)]


def search_fts(
    conn: sqlite3.Connection,
    query: str,
    limit: int = 50,
) -> list[dict]:
    conn.row_factory = sqlite3.Row
    sql = """
        SELECT e.file_id, e.type, e.name,
               snippet(entities_fts, 0, '<<', '>>', '...', 16) AS snippet
        FROM entities_fts
        JOIN entities e ON e.rowid = entities_fts.rowid
        WHERE entities_fts MATCH ?
        LIMIT ?
    """
    return [_row_to_dict(row) for row in conn.execute(sql, (query, limit))]


def backlinks(conn: sqlite3.Connection, file_id: str) -> list[dict]:
    conn.row_factory = sqlite3.Row
    sql = """
        SELECT src_file_id, field
        FROM entity_links
        WHERE resolved = ?
        ORDER BY src_file_id
    """
    return [_row_to_dict(row) for row in conn.execute(sql, (file_id,))]
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_index/test_reader.py -v
```
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/index/reader.py sidecar/tests/test_index/test_reader.py
git commit -m "feat(sidecar): index reader — list/filter/search/backlinks"
```

---

## Task 18: Reconciliation (mtime sweep + full rebuild)

**Files:**
- Create: `sidecar/src/eduport/index/reconcile.py`
- Test: `sidecar/tests/test_index/test_reconcile.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_index/test_reconcile.py`:
```python
import sqlite3
from pathlib import Path

import pytest

from eduport.index.reconcile import reconcile
from eduport.index.schema import init_schema


def _write(folder: Path, name: str, content: str) -> Path:
    p = folder / name
    p.write_text(content, encoding="utf-8")
    return p


@pytest.fixture
def conn_and_folder(tmp_path):
    folder = tmp_path / "data"
    folder.mkdir()
    conn = sqlite3.connect(tmp_path / "x.db")
    init_schema(conn)
    return conn, folder


def test_reconcile_inserts_new_files(conn_and_folder):
    conn, folder = conn_and_folder
    _write(folder, "eth-K9p3.md", """---
tags: [eduport-type/university]
name: ETH
country: CH
---
""")
    summary = reconcile(conn, folder)
    assert summary.added == 1
    assert summary.errors == 0
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 1


def test_reconcile_records_parse_errors(conn_and_folder):
    conn, folder = conn_and_folder
    _write(folder, "bad-X1x1.md", "no frontmatter at all")
    summary = reconcile(conn, folder)
    assert summary.errors == 1
    assert conn.execute("SELECT COUNT(*) FROM parse_errors").fetchone()[0] == 1


def test_reconcile_removes_missing_files(conn_and_folder):
    conn, folder = conn_and_folder
    f = _write(folder, "eth-K9p3.md", """---
tags: [eduport-type/university]
name: ETH
country: CH
---
""")
    reconcile(conn, folder)
    f.unlink()
    summary = reconcile(conn, folder)
    assert summary.removed == 1
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 0
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_index/test_reconcile.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/index/reconcile.py`:
```python
from __future__ import annotations

import logging
import sqlite3
from dataclasses import dataclass
from pathlib import Path

from eduport.index.writer import (
    clear_parse_error,
    delete_entity,
    record_parse_error,
    upsert_entity,
)
from eduport.parsers.entity import ParsedEntity, ParseError, parse_file

log = logging.getLogger("eduport.reconcile")


@dataclass
class ReconcileSummary:
    added: int = 0
    updated: int = 0
    removed: int = 0
    unchanged: int = 0
    errors: int = 0


def reconcile(conn: sqlite3.Connection, data_folder: Path) -> ReconcileSummary:
    summary = ReconcileSummary()

    # Existing index state
    existing: dict[str, int] = {
        row[0]: row[1]
        for row in conn.execute("SELECT file_id, mtime_ns FROM entities")
    }

    seen_ids: set[str] = set()
    for path in sorted(data_folder.glob("*.md")):
        # Skip dotfiles
        if path.name.startswith("."):
            continue
        file_id = path.stem
        seen_ids.add(file_id)
        try:
            mtime_ns = path.stat().st_mtime_ns
        except OSError as exc:
            log.warning("stat failed for %s: %s", path, exc)
            continue

        if existing.get(file_id) == mtime_ns:
            summary.unchanged += 1
            continue

        result = parse_file(path)
        if isinstance(result, ParseError):
            record_parse_error(conn, str(path), result.message)
            summary.errors += 1
            continue

        upsert_entity(
            conn,
            file_id=file_id,
            path=result.path,
            mtime_ns=mtime_ns,
            entity=result.entity,
            body=result.body,
        )
        clear_parse_error(conn, str(path))
        if file_id in existing:
            summary.updated += 1
        else:
            summary.added += 1

    # Remove files that no longer exist
    for file_id in set(existing) - seen_ids:
        delete_entity(conn, file_id)
        summary.removed += 1

    log.info("reconcile: %s", summary)
    return summary
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_index/test_reconcile.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/index/reconcile.py sidecar/tests/test_index/test_reconcile.py
git commit -m "feat(sidecar): mtime-based folder reconciliation with parse-error capture"
```

---

## Task 19: File store — write + delete with re-entrancy guard

**Files:**
- Create: `sidecar/src/eduport/store/__init__.py`
- Create: `sidecar/src/eduport/store/files.py`
- Test: `sidecar/tests/test_store/__init__.py` and `test_files.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_store/__init__.py`: empty.

`sidecar/tests/test_store/test_files.py`:
```python
from pathlib import Path

from eduport.models import University
from eduport.store.files import (
    EntityFileStore,
    serialize_entity_to_markdown,
)


def test_serialize_entity_round_trip():
    uni = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH",
        "country": "Switzerland",
    })
    text = serialize_entity_to_markdown(uni, body="My notes")
    assert text.startswith("---")
    assert "eduport-type/university" in text
    assert text.rstrip().endswith("My notes")


def test_write_creates_file(tmp_path: Path):
    store = EntityFileStore(tmp_path)
    uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    path = store.write("eth-K9p3", uni, body="")
    assert path == tmp_path / "eth-K9p3.md"
    assert path.exists()
    assert "eduport-type/university" in path.read_text()


def test_re_entrancy_guard_marks_writes(tmp_path: Path):
    store = EntityFileStore(tmp_path)
    uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    path = store.write("eth-K9p3", uni, body="")
    assert store.was_recently_written(path) is True
    # Marker auto-clears after the next call to was_recently_written that consumes it
    assert store.was_recently_written(path) is False
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_store/test_files.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/store/__init__.py`: empty.

`sidecar/src/eduport/store/files.py`:
```python
from __future__ import annotations

import json
from pathlib import Path

import yaml

from eduport.models import BaseEntity


def serialize_entity_to_markdown(entity: BaseEntity, body: str) -> str:
    """Render entity to '---\\nYAML\\n---\\n\\nbody' format."""
    payload = json.loads(entity.model_dump_json(by_alias=True))
    yaml_block = yaml.safe_dump(payload, sort_keys=False, allow_unicode=True).strip()
    body_stripped = body.lstrip("\n")
    return f"---\n{yaml_block}\n---\n\n{body_stripped}".rstrip() + "\n"


class EntityFileStore:
    """Owns writes to .md files. Tracks recent writes to suppress watcher feedback."""

    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self._recent_writes: set[Path] = set()

    def write(self, file_id: str, entity: BaseEntity, body: str) -> Path:
        path = self.data_folder / f"{file_id}.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(serialize_entity_to_markdown(entity, body), encoding="utf-8")
        self._recent_writes.add(path.resolve())
        return path

    def delete_marker(self, path: Path) -> None:
        """Mark a path as recently-touched, even though we didn't write to it.
        Used by the soft-delete flow."""
        self._recent_writes.add(path.resolve())

    def was_recently_written(self, path: Path) -> bool:
        """Return True (and consume the marker) if `path` was just written by this store."""
        resolved = path.resolve()
        if resolved in self._recent_writes:
            self._recent_writes.discard(resolved)
            return True
        return False
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_store/test_files.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/store/ sidecar/tests/test_store/
git commit -m "feat(sidecar): EntityFileStore with re-entrancy markers"
```

---

## Task 20: Trash — send2trash + .eduport-trash fallback

**Files:**
- Create: `sidecar/src/eduport/store/trash.py`
- Test: `sidecar/tests/test_store/test_trash.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_store/test_trash.py`:
```python
from pathlib import Path

from eduport.store.trash import LocalTrash


def test_local_trash_moves_to_subfolder(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f = folder / "eth-K9p3.md"
    f.write_text("body")

    trash = LocalTrash(folder)
    moved = trash.trash(f)
    assert not f.exists()
    assert moved.exists()
    assert moved.parent.name == ".eduport-trash"
    assert moved.read_text() == "body"


def test_local_trash_handles_name_collision(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f1 = folder / "eth.md"
    f1.write_text("v1")

    trash = LocalTrash(folder)
    first = trash.trash(f1)

    f2 = folder / "eth.md"
    f2.write_text("v2")
    second = trash.trash(f2)

    assert first.exists()
    assert second.exists()
    assert first != second
    assert {first.read_text(), second.read_text()} == {"v1", "v2"}


def test_restore_returns_to_original(tmp_path: Path):
    folder = tmp_path / "data"
    folder.mkdir()
    f = folder / "eth.md"
    f.write_text("body")
    trash = LocalTrash(folder)
    moved = trash.trash(f)

    restored = trash.restore(moved)
    assert restored == f
    assert restored.exists()
    assert not moved.exists()
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_store/test_trash.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/store/trash.py`:
```python
from __future__ import annotations

from pathlib import Path
from typing import Optional


class LocalTrash:
    """Move-to-trash with a per-data-folder `.eduport-trash/` subdirectory.

    Each trashed file's original parent path is encoded in a sidecar metadata file
    so we can restore later. For v1 we keep the metadata simple.
    """

    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self.trash_dir = data_folder / ".eduport-trash"

    def trash(self, path: Path) -> Path:
        self.trash_dir.mkdir(parents=True, exist_ok=True)
        target = self._unique_destination(path.name)
        path.rename(target)
        # Sidecar JSON for restore
        meta = target.with_suffix(target.suffix + ".restore-from")
        meta.write_text(str(path), encoding="utf-8")
        return target

    def restore(self, trashed: Path) -> Path:
        meta = trashed.with_suffix(trashed.suffix + ".restore-from")
        if not meta.exists():
            raise FileNotFoundError(f"no restore metadata for {trashed}")
        original = Path(meta.read_text(encoding="utf-8"))
        original.parent.mkdir(parents=True, exist_ok=True)
        trashed.rename(original)
        meta.unlink()
        return original

    def _unique_destination(self, name: str) -> Path:
        candidate = self.trash_dir / name
        if not candidate.exists():
            return candidate
        stem, suffix = candidate.stem, candidate.suffix
        i = 1
        while True:
            alt = self.trash_dir / f"{stem}.{i}{suffix}"
            if not alt.exists():
                return alt
            i += 1
```

For OS-trash integration (nice-to-have, not needed for the test): wrap with `send2trash` and fall back to `LocalTrash` on failure. This is exposed via a separate function:

Append to `sidecar/src/eduport/store/trash.py`:
```python
import logging
import send2trash

log = logging.getLogger("eduport.trash")


def trash_with_fallback(path: Path, fallback: LocalTrash) -> Optional[Path]:
    """Move to OS trash via send2trash; fall back to LocalTrash on error.
    Returns the path inside the local fallback if used, None when OS trash succeeded.
    """
    try:
        send2trash.send2trash(str(path))
        return None
    except (OSError, send2trash.TrashPermissionError) as exc:
        log.warning("OS trash failed for %s: %s — falling back to local", path, exc)
        return fallback.trash(path)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_store/test_trash.py -v
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/store/trash.py sidecar/tests/test_store/test_trash.py
git commit -m "feat(sidecar): local trash with restore + OS-trash fallback"
```

---

## Task 21: File watcher — debounced watchdog

**Files:**
- Create: `sidecar/src/eduport/watcher.py`
- Test: `sidecar/tests/test_watcher.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_watcher.py`:
```python
import time
from pathlib import Path

import pytest

from eduport.watcher import EduportWatcher


@pytest.fixture
def folder(tmp_path: Path) -> Path:
    return tmp_path


def _wait(predicate, timeout: float = 2.0) -> None:
    end = time.time() + timeout
    while time.time() < end:
        if predicate():
            return
        time.sleep(0.05)
    raise AssertionError("predicate never became true")


def test_watcher_fires_on_create_modify_delete(folder):
    seen: list[tuple[str, str]] = []

    def on_event(kind: str, path: Path) -> None:
        seen.append((kind, path.name))

    watcher = EduportWatcher(folder, on_event)
    watcher.start()
    try:
        f = folder / "x-Y2y2.md"
        f.write_text("hello")
        _wait(lambda: any(k == "created" and n == "x-Y2y2.md" for k, n in seen))

        f.write_text("hello again")
        _wait(lambda: any(k == "modified" and n == "x-Y2y2.md" for k, n in seen))

        f.unlink()
        _wait(lambda: any(k == "deleted" and n == "x-Y2y2.md" for k, n in seen))
    finally:
        watcher.stop()
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_watcher.py -v
```
Expected: ImportError.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/watcher.py`:
```python
from __future__ import annotations

import logging
from pathlib import Path
from typing import Callable

from watchdog.events import FileSystemEvent, FileSystemEventHandler
from watchdog.observers import Observer

log = logging.getLogger("eduport.watcher")

EventCallback = Callable[[str, Path], None]


class _Handler(FileSystemEventHandler):
    def __init__(self, callback: EventCallback) -> None:
        self.callback = callback

    def _emit(self, kind: str, raw_path: str) -> None:
        path = Path(raw_path)
        if path.suffix != ".md" or path.name.startswith("."):
            return
        self.callback(kind, path)

    def on_created(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("created", event.src_path)

    def on_modified(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("modified", event.src_path)

    def on_deleted(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("deleted", event.src_path)


class EduportWatcher:
    def __init__(self, folder: Path, callback: EventCallback) -> None:
        self.folder = folder
        self.callback = callback
        self._observer: Observer | None = None  # type: ignore[type-arg]

    def start(self) -> None:
        if self._observer is not None:
            return
        observer = Observer()
        observer.schedule(_Handler(self.callback), str(self.folder), recursive=False)
        observer.start()
        self._observer = observer
        log.info("watcher started on %s", self.folder)

    def stop(self) -> None:
        if self._observer is None:
            return
        self._observer.stop()
        self._observer.join(timeout=2.0)
        self._observer = None
        log.info("watcher stopped")
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_watcher.py -v
```
Expected: 1 passed (may take ~1-2s due to filesystem polling on some platforms).

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/watcher.py sidecar/tests/test_watcher.py
git commit -m "feat(sidecar): watchdog-based file watcher"
```

---

## Task 22: API — app factory, dependencies, health endpoint

**Files:**
- Create: `sidecar/src/eduport/api/__init__.py`
- Create: `sidecar/src/eduport/api/deps.py`
- Create: `sidecar/src/eduport/api/health.py`
- Create: `sidecar/src/eduport/api/app.py`
- Create: `sidecar/tests/test_api/__init__.py`
- Create: `sidecar/tests/conftest.py`
- Test: `sidecar/tests/test_api/test_health.py`

- [ ] **Step 1: Write the failing test + fixtures**

`sidecar/tests/test_api/__init__.py`: empty.

`sidecar/tests/conftest.py`:
```python
import sqlite3
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.settings import Settings


@pytest.fixture
def settings(tmp_path: Path) -> Settings:
    data = tmp_path / "data"
    data.mkdir()
    (data / "attachments").mkdir()
    (data / "notes").mkdir()
    return Settings(
        data_folder=data,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )


@pytest.fixture
def conn(tmp_path: Path) -> sqlite3.Connection:
    c = sqlite3.connect(tmp_path / "index.db")
    init_schema(c)
    return c


@pytest.fixture
def client(settings: Settings, conn: sqlite3.Connection) -> TestClient:
    app = build_app(settings=settings, conn=conn, start_watcher=False)
    return TestClient(app)
```

`sidecar/tests/test_api/test_health.py`:
```python
def test_health(client):
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_health.py -v
```
Expected: ImportError on `eduport.api.app`.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/__init__.py`: empty.

`sidecar/src/eduport/api/deps.py`:
```python
from __future__ import annotations

import sqlite3
from dataclasses import dataclass

from fastapi import Request

from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.trash import LocalTrash


@dataclass
class AppState:
    settings: Settings
    conn: sqlite3.Connection
    file_store: EntityFileStore
    trash: LocalTrash


def get_state(request: Request) -> AppState:
    return request.app.state.eduport
```

`sidecar/src/eduport/api/health.py`:
```python
from fastapi import APIRouter

router = APIRouter()


@router.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}
```

`sidecar/src/eduport/api/app.py`:
```python
from __future__ import annotations

import sqlite3

from fastapi import FastAPI

from eduport.api.deps import AppState
from eduport.api.health import router as health_router
from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.trash import LocalTrash


def build_app(
    settings: Settings,
    conn: sqlite3.Connection,
    start_watcher: bool = True,
) -> FastAPI:
    app = FastAPI(title="Eduport sidecar", version="0.1.0")
    app.state.eduport = AppState(
        settings=settings,
        conn=conn,
        file_store=EntityFileStore(settings.data_folder),
        trash=LocalTrash(settings.data_folder),
    )
    app.include_router(health_router)
    return app
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_health.py -v
```
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/ sidecar/tests/conftest.py sidecar/tests/test_api/
git commit -m "feat(sidecar): FastAPI app factory with health endpoint"
```

---

## Task 23: API — list and get endpoints

**Files:**
- Create: `sidecar/src/eduport/api/entities.py`
- Modify: `sidecar/src/eduport/api/app.py` (mount router)
- Test: `sidecar/tests/test_api/test_entities.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_entities.py`:
```python
from pathlib import Path

import pytest

from eduport.index.writer import upsert_entity
from eduport.models import University


@pytest.fixture
def seeded_client(client, conn):
    eth = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH", "country": "Switzerland",
    })
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, eth, "Body")
    return client


def test_list_universities(seeded_client):
    response = seeded_client.get("/entities/university")
    assert response.status_code == 200
    payload = response.json()
    assert len(payload) == 1
    assert payload[0]["file_id"] == "eth-K9p3"


def test_list_with_tag_filter(seeded_client):
    response = seeded_client.get("/entities/university?tag=switzerland")
    assert response.status_code == 200
    assert len(response.json()) == 1


def test_list_unknown_type_returns_400(client):
    response = client.get("/entities/imaginary")
    assert response.status_code == 400


def test_get_one(seeded_client):
    response = seeded_client.get("/entities/university/eth-K9p3")
    assert response.status_code == 200
    body = response.json()
    assert body["entity"]["name"] == "ETH"
    assert "body" in body
    assert "backlinks" in body


def test_get_missing_returns_404(client):
    response = client.get("/entities/university/ghost-Z9z9")
    assert response.status_code == 404
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_entities.py -v
```
Expected: ImportError or 404 from missing routes.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/entities.py`:
```python
from __future__ import annotations

import json
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from eduport.api.deps import AppState, get_state
from eduport.index.reader import backlinks, list_entities
from eduport.models import EntityType

router = APIRouter(prefix="/entities", tags=["entities"])


def _validate_type(type_: str) -> str:
    try:
        return EntityType(type_).value
    except ValueError:
        raise HTTPException(status_code=400, detail=f"unknown type: {type_!r}")


@router.get("/{type_}")
def list_(
    type_: str,
    tag: Annotated[Optional[list[str]], Query()] = None,
    state: AppState = Depends(get_state),
) -> list[dict]:
    type_ = _validate_type(type_)
    return list_entities(state.conn, type=type_, tags=tag or [])


@router.get("/{type_}/{file_id}")
def get_one(
    type_: str,
    file_id: str,
    state: AppState = Depends(get_state),
) -> dict:
    type_ = _validate_type(type_)
    row = state.conn.execute(
        "SELECT type, name, body, frontmatter FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_),
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="not found")
    return {
        "file_id": file_id,
        "type": row[0],
        "entity": json.loads(row[3]),
        "body": row[2],
        "backlinks": backlinks(state.conn, file_id),
    }
```

Modify `sidecar/src/eduport/api/app.py` to include the router:
```python
# add to imports
from eduport.api.entities import router as entities_router
# add to app setup, after health_router:
app.include_router(entities_router)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_entities.py -v
```
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/entities.py sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_entities.py
git commit -m "feat(sidecar): /entities list + get endpoints"
```

---

## Task 24: API — create / update / delete entity

**Files:**
- Modify: `sidecar/src/eduport/api/entities.py`
- Test: append to `sidecar/tests/test_api/test_entities.py`

- [ ] **Step 1: Write the failing tests (append)**

```python
def test_create_entity(client, settings):
    payload = {
        "tags": ["eduport-type/university", "ai"],
        "name": "MIT",
        "country": "USA",
    }
    response = client.post(
        "/entities/university",
        json={"frontmatter": payload, "body": "Notes about MIT."},
    )
    assert response.status_code == 201
    file_id = response.json()["file_id"]
    assert file_id.startswith("mit-")
    # Disk artifact check
    assert (settings.data_folder / f"{file_id}.md").exists()


def test_update_entity(seeded_client):
    response = seeded_client.patch(
        "/entities/university/eth-K9p3",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university"],
                "name": "ETH (renamed)",
                "country": "Switzerland",
            },
            "body": "Updated notes.",
        },
    )
    assert response.status_code == 200
    after = seeded_client.get("/entities/university/eth-K9p3").json()
    assert after["entity"]["name"] == "ETH (renamed)"
    assert after["body"] == "Updated notes."


def test_delete_moves_to_trash(seeded_client, settings):
    response = seeded_client.delete("/entities/university/eth-K9p3")
    assert response.status_code == 204
    # Index gone
    assert seeded_client.get("/entities/university/eth-K9p3").status_code == 404
    # File gone from main folder, present in trash
    assert not (settings.data_folder / "eth-K9p3.md").exists()
    trashed = list((settings.data_folder / ".eduport-trash").glob("eth-K9p3*"))
    assert trashed
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_entities.py -v
```
Expected: 405 method-not-allowed errors.

- [ ] **Step 3: Append minimal implementation to `sidecar/src/eduport/api/entities.py`**

```python
from pathlib import Path

from pydantic import BaseModel

from eduport.ids import generate_id
from eduport.index.writer import delete_entity, upsert_entity
from eduport.models.base import BaseEntity
from eduport.parsers.entity import _TYPE_TO_MODEL  # private but cohesive
from eduport.slug import generate_slug


class EntityWriteIn(BaseModel):
    frontmatter: dict
    body: str = ""


@router.post("/{type_}", status_code=201)
def create(
    type_: str,
    payload: EntityWriteIn,
    state: AppState = Depends(get_state),
) -> dict:
    type_value = _validate_type(type_)
    model_cls = _TYPE_TO_MODEL[EntityType(type_value)]
    try:
        entity: BaseEntity = model_cls.model_validate(payload.frontmatter)
    except Exception as exc:
        raise HTTPException(status_code=422, detail=str(exc))

    slug = generate_slug(entity.name)
    existing_ids = {
        row[0] for row in state.conn.execute("SELECT file_id FROM entities")
    }
    new_id = generate_id(lambda candidate: f"{slug}-{candidate}" in existing_ids)
    file_id = f"{slug}-{new_id}"

    path = state.file_store.write(file_id, entity, payload.body)
    upsert_entity(
        state.conn,
        file_id=file_id,
        path=path,
        mtime_ns=path.stat().st_mtime_ns,
        entity=entity,
        body=payload.body,
    )
    return {"file_id": file_id}


@router.patch("/{type_}/{file_id}")
def update(
    type_: str,
    file_id: str,
    payload: EntityWriteIn,
    state: AppState = Depends(get_state),
) -> dict:
    type_value = _validate_type(type_)
    if not state.conn.execute(
        "SELECT 1 FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_value),
    ).fetchone():
        raise HTTPException(status_code=404, detail="not found")

    model_cls = _TYPE_TO_MODEL[EntityType(type_value)]
    try:
        entity: BaseEntity = model_cls.model_validate(payload.frontmatter)
    except Exception as exc:
        raise HTTPException(status_code=422, detail=str(exc))

    path = state.file_store.write(file_id, entity, payload.body)
    upsert_entity(
        state.conn,
        file_id=file_id,
        path=path,
        mtime_ns=path.stat().st_mtime_ns,
        entity=entity,
        body=payload.body,
    )
    return {"file_id": file_id}


@router.delete("/{type_}/{file_id}", status_code=204)
def delete(
    type_: str,
    file_id: str,
    state: AppState = Depends(get_state),
) -> None:
    type_value = _validate_type(type_)
    row = state.conn.execute(
        "SELECT path FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_value),
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="not found")
    path = Path(row[0])
    if path.exists():
        state.trash.trash(path)
        state.file_store.delete_marker(path)
    delete_entity(state.conn, file_id)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_entities.py -v
```
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/entities.py sidecar/tests/test_api/test_entities.py
git commit -m "feat(sidecar): /entities create + update + delete"
```

---

## Task 25: API — search

**Files:**
- Create: `sidecar/src/eduport/api/search.py`
- Modify: `sidecar/src/eduport/api/app.py`
- Test: `sidecar/tests/test_api/test_search.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_search.py`:
```python
from pathlib import Path

from eduport.index.writer import upsert_entity
from eduport.models import University


def test_search_finds_in_body(client, conn):
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "CH",
    })
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, eth, "Strong AI track here")
    response = client.get("/search?q=track")
    assert response.status_code == 200
    hits = response.json()
    assert len(hits) == 1
    assert hits[0]["file_id"] == "eth-K9p3"
    assert "snippet" in hits[0]


def test_search_no_results(client):
    response = client.get("/search?q=zzznothing")
    assert response.status_code == 200
    assert response.json() == []
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_search.py -v
```
Expected: 404 (no /search route).

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/search.py`:
```python
from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state
from eduport.index.reader import search_fts

router = APIRouter()


@router.get("/search")
def search(q: str, limit: int = 50, state: AppState = Depends(get_state)) -> list[dict]:
    if not q.strip():
        return []
    return search_fts(state.conn, q, limit=limit)
```

Modify `sidecar/src/eduport/api/app.py`:
```python
from eduport.api.search import router as search_router
# ...
app.include_router(search_router)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_search.py -v
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/search.py sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_search.py
git commit -m "feat(sidecar): /search endpoint backed by FTS5"
```

---

## Task 26: API — checkbox toggle

**Files:**
- Create: `sidecar/src/eduport/api/checkbox.py`
- Modify: `sidecar/src/eduport/api/app.py`
- Test: `sidecar/tests/test_api/test_checkbox.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_checkbox.py`:
```python
from pathlib import Path

from eduport.index.writer import upsert_entity
from eduport.models import Application


def test_toggle_checkbox(client, conn, settings):
    body = "Note\n- [ ] task one\n- [ ] task two\n"
    app_entity = Application.model_validate({
        "tags": ["eduport-type/application"],
        "name": "ETH 2026",
        "program": "[[msc-cs-Q7w8]]",
        "status": "drafting",
    })
    # Simulate the file existing on disk so the API can rewrite it
    path = settings.data_folder / "app-A1a1.md"
    path.write_text(f"---\ntags: [eduport-type/application]\nname: ETH 2026\nprogram: \"[[msc-cs-Q7w8]]\"\nstatus: drafting\n---\n\n{body}", encoding="utf-8")
    upsert_entity(conn, "app-A1a1", path, path.stat().st_mtime_ns, app_entity, body)

    response = client.post(
        "/checkbox/toggle",
        json={"file_id": "app-A1a1", "line": 1, "checked": True},
    )
    assert response.status_code == 200

    new_text = path.read_text(encoding="utf-8")
    assert "- [x] task one" in new_text
    assert "- [ ] task two" in new_text
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_checkbox.py -v
```
Expected: 404 (no route).

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/checkbox.py`:
```python
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from eduport.api.deps import AppState, get_state

router = APIRouter()


class ToggleIn(BaseModel):
    file_id: str
    line: int
    checked: bool


@router.post("/checkbox/toggle")
def toggle(payload: ToggleIn, state: AppState = Depends(get_state)) -> dict:
    row = state.conn.execute(
        "SELECT path FROM entities WHERE file_id = ?", (payload.file_id,)
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="entity not found")
    path = Path(row[0])
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    # Body lines start after the closing ---. Find it.
    body_start = 0
    if lines and lines[0] == "---":
        for i in range(1, len(lines)):
            if lines[i] == "---":
                body_start = i + 1
                # skip blank line if present
                if body_start < len(lines) and lines[body_start] == "":
                    body_start += 1
                break
    target = body_start + payload.line
    if target >= len(lines):
        raise HTTPException(status_code=400, detail="line out of range")
    line = lines[target]
    new_marker = "[x]" if payload.checked else "[ ]"
    if line.startswith("- [ ]"):
        lines[target] = "- " + new_marker + line[len("- [ ]"):]
    elif line.startswith("- [x]") or line.startswith("- [X]"):
        lines[target] = "- " + new_marker + line[len("- [x]"):]
    else:
        raise HTTPException(status_code=400, detail="line is not a checkbox")
    new_text = "\n".join(lines) + ("\n" if text.endswith("\n") else "")
    path.write_text(new_text, encoding="utf-8")
    state.file_store.delete_marker(path)
    return {"ok": True}
```

Modify `sidecar/src/eduport/api/app.py` to include the router:
```python
from eduport.api.checkbox import router as checkbox_router
# ...
app.include_router(checkbox_router)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_checkbox.py -v
```
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/checkbox.py sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_checkbox.py
git commit -m "feat(sidecar): /checkbox/toggle endpoint"
```

---

## Task 27: API — `.eml` parse

**Files:**
- Create: `sidecar/src/eduport/api/eml_import.py`
- Modify: `sidecar/src/eduport/api/app.py`
- Test: `sidecar/tests/test_api/test_eml_import.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_eml_import.py`:
```python
from pathlib import Path


def test_eml_parse_returns_form_payload(client):
    fixture = Path(__file__).parent.parent / "fixtures" / "sample.eml"
    response = client.post(
        "/eml/parse",
        files={"file": ("sample.eml", fixture.read_bytes(), "message/rfc822")},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["from"] == "jane@example.com"
    assert body["subject"] == "Welcome to ETH"
    assert "Welcome to the program" in body["body"]
    assert body["direction"] == "inbound"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_eml_import.py -v
```
Expected: 404.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/eml_import.py`:
```python
from fastapi import APIRouter, Depends, File, UploadFile

from eduport.api.deps import AppState, get_state
from eduport.parsers.eml import parse_eml

router = APIRouter()


@router.post("/eml/parse")
async def eml_parse(
    file: UploadFile = File(...),
    state: AppState = Depends(get_state),
) -> dict:
    payload = await file.read()
    parsed = parse_eml(payload, user_email=state.settings.user_email)
    return {
        "from": parsed.from_,
        "to": parsed.to,
        "cc": parsed.cc,
        "bcc": parsed.bcc,
        "subject": parsed.subject,
        "date": parsed.date.isoformat() if parsed.date else None,
        "body": parsed.body,
        "direction": parsed.direction.value,
    }
```

Modify `sidecar/src/eduport/api/app.py`:
```python
from eduport.api.eml_import import router as eml_router
# ...
app.include_router(eml_router)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_eml_import.py -v
```
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/eml_import.py sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_eml_import.py
git commit -m "feat(sidecar): /eml/parse endpoint"
```

---

## Task 28: API — settings (GET / PUT)

**Files:**
- Create: `sidecar/src/eduport/api/settings_api.py`
- Modify: `sidecar/src/eduport/api/app.py`
- Test: `sidecar/tests/test_api/test_settings_api.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_settings_api.py`:
```python
def test_get_settings(client):
    response = client.get("/settings")
    assert response.status_code == 200
    body = response.json()
    assert "data_folder" in body
    assert body["theme"] == "system"


def test_put_settings(client):
    new_payload = {
        "data_folder": "/tmp/new-data",
        "attachments_folder": "./attachments",
        "notes_folder": "./notes",
        "theme": "dark",
        "user_email": "rusen@example.com",
    }
    response = client.put("/settings", json=new_payload)
    assert response.status_code == 200
    assert response.json()["theme"] == "dark"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_settings_api.py -v
```
Expected: 404.

- [ ] **Step 3: Write minimal implementation**

`sidecar/src/eduport/api/settings_api.py`:
```python
from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state
from eduport.settings import Settings

router = APIRouter()


@router.get("/settings")
def get_settings(state: AppState = Depends(get_state)) -> dict:
    payload = state.settings.model_dump(mode="json")
    payload["data_folder"] = str(payload["data_folder"])
    return payload


@router.put("/settings")
def put_settings(payload: Settings, state: AppState = Depends(get_state)) -> dict:
    # Mutate state in place. Persistence to disk is the CLI/launcher's job (Plan 3).
    state.settings = payload
    out = payload.model_dump(mode="json")
    out["data_folder"] = str(out["data_folder"])
    return out
```

Modify `sidecar/src/eduport/api/app.py`:
```python
from eduport.api.settings_api import router as settings_router
# ...
app.include_router(settings_router)
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_settings_api.py -v
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/settings_api.py sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_settings_api.py
git commit -m "feat(sidecar): /settings get + put endpoints"
```

---

## Task 29: Wire watcher + reconcile into app lifespan

**Files:**
- Modify: `sidecar/src/eduport/api/app.py`
- Test: `sidecar/tests/test_api/test_lifespan.py`

- [ ] **Step 1: Write the failing test**

`sidecar/tests/test_api/test_lifespan.py`:
```python
from pathlib import Path

from fastapi.testclient import TestClient

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.settings import Settings


def test_lifespan_runs_initial_reconcile(tmp_path: Path):
    data = tmp_path / "data"
    data.mkdir()
    (data / "attachments").mkdir()
    (data / "notes").mkdir()
    (data / "eth-K9p3.md").write_text("""---
tags: [eduport-type/university]
name: ETH
country: CH
---
""", encoding="utf-8")
    settings = Settings(
        data_folder=data,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )
    import sqlite3
    conn = sqlite3.connect(":memory:")
    init_schema(conn)
    app = build_app(settings=settings, conn=conn, start_watcher=False, run_reconcile=True)
    with TestClient(app):
        resp = TestClient(app).get("/entities/university")
        assert resp.status_code == 200
        names = [r["name"] for r in resp.json()]
        assert "ETH" in names
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd sidecar && uv run pytest tests/test_api/test_lifespan.py -v
```
Expected: TypeError or empty list — `run_reconcile` parameter unknown.

- [ ] **Step 3: Modify implementation**

Update `sidecar/src/eduport/api/app.py`:
```python
from contextlib import asynccontextmanager
from pathlib import Path

from eduport.index.reconcile import reconcile
from eduport.watcher import EduportWatcher


def build_app(
    settings: Settings,
    conn: sqlite3.Connection,
    start_watcher: bool = True,
    run_reconcile: bool = True,
) -> FastAPI:
    @asynccontextmanager
    async def lifespan(app: FastAPI):
        if run_reconcile:
            reconcile(app.state.eduport.conn, settings.data_folder)
        watcher: EduportWatcher | None = None
        if start_watcher:
            def on_event(kind: str, path: Path) -> None:
                from eduport.parsers.entity import ParsedEntity, ParseError, parse_file
                from eduport.index.writer import (
                    delete_entity, record_parse_error, upsert_entity, clear_parse_error,
                )
                if app.state.eduport.file_store.was_recently_written(path):
                    return
                if kind == "deleted":
                    delete_entity(app.state.eduport.conn, path.stem)
                    return
                result = parse_file(path)
                if isinstance(result, ParseError):
                    record_parse_error(app.state.eduport.conn, str(path), result.message)
                    return
                upsert_entity(
                    app.state.eduport.conn,
                    file_id=path.stem,
                    path=path,
                    mtime_ns=path.stat().st_mtime_ns,
                    entity=result.entity,
                    body=result.body,
                )
                clear_parse_error(app.state.eduport.conn, str(path))

            watcher = EduportWatcher(settings.data_folder, on_event)
            watcher.start()
        try:
            yield
        finally:
            if watcher is not None:
                watcher.stop()

    app = FastAPI(title="Eduport sidecar", version="0.1.0", lifespan=lifespan)
    # ...rest unchanged
```

(Keep the rest of `build_app` body, including the routers.)

- [ ] **Step 4: Run test to verify it passes**

```bash
cd sidecar && uv run pytest tests/test_api/test_lifespan.py -v
```
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/api/app.py sidecar/tests/test_api/test_lifespan.py
git commit -m "feat(sidecar): lifespan reconcile + watcher wiring"
```

---

## Task 30: CLI entry point

**Files:**
- Create: `sidecar/src/eduport/cli.py`
- Modify: `sidecar/pyproject.toml` (add script entry)

- [ ] **Step 1: Add the entry-point script**

Append to `sidecar/pyproject.toml` under `[project]`:
```toml
[project.scripts]
eduport-sidecar = "eduport.cli:main"
```

- [ ] **Step 2: Write CLI module**

`sidecar/src/eduport/cli.py`:
```python
from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path

import platformdirs
import uvicorn

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.logging_setup import configure_logging
from eduport.settings import Settings, load_settings


def _index_path(data_folder: Path) -> Path:
    cache_dir = Path(platformdirs.user_cache_dir("Eduport", appauthor=False))
    cache_dir.mkdir(parents=True, exist_ok=True)
    folder_hash = abs(hash(str(data_folder.resolve()))) % (2**32)
    return cache_dir / f"index-{folder_hash:08x}.sqlite"


def _log_path() -> Path:
    log_dir = Path(platformdirs.user_log_dir("Eduport", appauthor=False))
    log_dir.mkdir(parents=True, exist_ok=True)
    return log_dir / "sidecar.log"


def _settings_path() -> Path:
    cfg = Path(platformdirs.user_config_dir("Eduport", appauthor=False))
    return cfg / "settings.toml"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="eduport-sidecar")
    parser.add_argument("--port", type=int, default=0, help="bind port (0 = random)")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--settings", type=Path, default=None, help="override settings path")
    args = parser.parse_args(argv)

    configure_logging(_log_path())
    settings_file = args.settings or _settings_path()
    settings = load_settings(settings_file)
    if settings is None:
        sys.stderr.write(
            f"No settings found at {settings_file}. The launcher should write one before starting the sidecar.\n"
        )
        return 2

    conn = sqlite3.connect(_index_path(settings.data_folder), check_same_thread=False)
    init_schema(conn)

    app = build_app(settings=settings, conn=conn, start_watcher=True, run_reconcile=True)
    uvicorn.run(app, host=args.host, port=args.port)
    return 0
```

- [ ] **Step 3: Smoke test the import path**

Run:
```bash
cd sidecar && uv run python -c "from eduport.cli import main; print(main)"
```
Expected: prints something like `<function main at 0x...>` with no errors.

- [ ] **Step 4: Verify the script works**

Run:
```bash
cd sidecar && uv run eduport-sidecar --help
```
Expected: prints argparse help.

- [ ] **Step 5: Commit**

```bash
git add sidecar/src/eduport/cli.py sidecar/pyproject.toml
git commit -m "feat(sidecar): CLI entry point with platformdirs paths"
```

---

## Task 31: End-to-end smoke test

**Files:**
- Create: `sidecar/tests/test_api/test_e2e.py`

- [ ] **Step 1: Write the e2e test**

`sidecar/tests/test_api/test_e2e.py`:
```python
def test_full_flow_create_read_search_delete(client):
    # Create a University
    create = client.post(
        "/entities/university",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university", "switzerland"],
                "name": "ETH Zurich",
                "country": "Switzerland",
            },
            "body": "Strong AI track here",
        },
    )
    assert create.status_code == 201
    file_id = create.json()["file_id"]

    # List
    listed = client.get("/entities/university").json()
    assert any(r["file_id"] == file_id for r in listed)

    # Get with backlinks
    one = client.get(f"/entities/university/{file_id}").json()
    assert one["entity"]["name"] == "ETH Zurich"
    assert one["body"] == "Strong AI track here"
    assert one["backlinks"] == []

    # Search
    hits = client.get("/search?q=track").json()
    assert any(h["file_id"] == file_id for h in hits)

    # Update
    upd = client.patch(
        f"/entities/university/{file_id}",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university"],
                "name": "ETH Zurich (renamed)",
                "country": "Switzerland",
            },
            "body": "Updated body without that keyword",
        },
    )
    assert upd.status_code == 200
    after = client.get(f"/entities/university/{file_id}").json()
    assert after["entity"]["name"] == "ETH Zurich (renamed)"

    # Search must reflect new body
    no_track = [h for h in client.get("/search?q=track").json() if h["file_id"] == file_id]
    assert no_track == []

    # Delete
    deleted = client.delete(f"/entities/university/{file_id}")
    assert deleted.status_code == 204
    assert client.get(f"/entities/university/{file_id}").status_code == 404
```

- [ ] **Step 2: Run test (expect pass — wires everything together)**

```bash
cd sidecar && uv run pytest tests/test_api/test_e2e.py -v
```
Expected: 1 passed. If it fails, the failure points at integration bugs across CRUD/search.

- [ ] **Step 3: Run the full test suite**

```bash
cd sidecar && uv run pytest -v
```
Expected: All tests pass (~50+ tests across ~30 test files).

- [ ] **Step 4: Run linters and type checker**

```bash
cd sidecar && uv run ruff check . && uv run mypy src/
```
Fix anything that surfaces. (mypy may complain about `watchdog` types; either install `types-watchdog` if it exists or add `# type: ignore[import-untyped]` to that import.)

- [ ] **Step 5: Commit**

```bash
git add sidecar/tests/test_api/test_e2e.py
git commit -m "test(sidecar): end-to-end smoke covering CRUD + search"
```

Then push:
```bash
git push
```

---

## Self-review

After writing this plan, verifying against the spec section-by-section:

| Spec section | Covered by tasks |
|---|---|
| §3.1 Folder layout | Task 22 (deps include data_folder), Task 30 (CLI resolves paths) |
| §3.2 File naming + slug rules | Task 3 (slug), Task 4 (ids), Task 24 (create endpoint composes them) |
| §3.3 Type discrimination | Task 5 (EntityType enum), Task 12 (dispatcher) |
| §3.4 Wikilink resolution | Task 10 |
| §3.5 Tags | Task 5 (BaseEntity.user_tags), Task 17 (filtering) |
| §3.6 SQLite cache + path | Task 14 (schema), Task 30 (path via platformdirs) |
| §3.7 Sync conflicts | Out of scope for sidecar — handled at file-system level |
| §3.8 Parse-error handling | Task 16 (parse_errors table), Task 18 (reconcile records errors), Task 29 (lifespan dispatches) |
| §4 Data model (8 entities) | Tasks 5-8 |
| §6.1 Bootstrap sequence | Plan 3 (Tauri shell). Health endpoint provided in Task 22. |
| §7.6 FTS5 search | Task 14 (schema), Task 17 (search_fts), Task 25 (endpoint) |
| §7.8 Soft-delete | Task 20 (LocalTrash), Task 24 (delete endpoint uses it) |
| §8.5 .eml import | Task 13 (parser), Task 27 (endpoint) |
| §10 Test strategy | Every task is TDD; ruff + mypy run in Task 31 |

Gaps the implementer should be aware of:
- **Restore from trash** is implemented in `LocalTrash.restore()` (Task 20) but no API endpoint exposes it. v1 milestone (§13) says "soft-delete with restorable Trash" — Plan 2 (frontend) will need a `/trash` endpoint. That's a small addition; treat as a stub now and add it when Plan 2 needs it.
- **`emails:` resource lists with optional `person:`** are in the model (Task 5: `EmailResource`) and validate fine; nothing further needed in the sidecar.
- **Recommendation request tracking via `status: requested`** — Task 8's Document model defaults `status` correctly. The frontend will surface this in the Dashboard (Plan 2).
- **Inline body editing (CodeMirror)** is purely a frontend concern (Plan 2). The sidecar's update endpoint (Task 24) accepts body changes; that's the contract.
- **Logging visibility UI** (§7.9) — sidecar writes to a log file (Task 30); the frontend reads/displays it (Plan 2 + Plan 3).
- **First-run / onboarding** (§7.7) — purely Tauri shell (Plan 3). The sidecar refuses to start without a settings file (Task 30 returns exit 2), which is the contract Plan 3 codes against.

No placeholders found. Type signatures consistent across tasks (e.g., `parse_file` returns `ParsedEntity | ParseError` everywhere; `upsert_entity` signature is the same in writer.py, reconcile.py, and the lifespan).

The plan is one cohesive unit: each task produces a runnable artifact (test passes), and the final task (E2E) exercises the whole stack.
