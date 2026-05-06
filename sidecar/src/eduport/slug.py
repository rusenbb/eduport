import re
import unicodedata

_NON_ALNUM = re.compile(r"[^a-z0-9]+")
_MAX_LEN = 60

# ASCII transliteration for common European Latin characters that have no
# NFKD decomposition. Anything not in this map falls through to the regex
# pass and becomes a hyphen — acceptable v1 behavior for non-Latin scripts.
_ASCII_FOLD = str.maketrans(
    {
        "ø": "o", "Ø": "O",
        "æ": "ae", "Æ": "AE",
        "œ": "oe", "Œ": "OE",
        "ß": "ss",
        "þ": "th", "Þ": "TH",
        "ð": "d", "Ð": "D",
        "ł": "l", "Ł": "L",
    }
)


def generate_slug(name: str) -> str:
    # Step 1: NFKD normalize and strip combining marks
    folded = unicodedata.normalize("NFKD", name)
    folded = "".join(c for c in folded if not unicodedata.combining(c))
    # Step 1b: ASCII-fold non-decomposable Latin letters (ø, æ, ß, ...)
    folded = folded.translate(_ASCII_FOLD)
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
