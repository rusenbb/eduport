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
