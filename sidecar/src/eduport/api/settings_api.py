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
    state.settings = payload
    out = payload.model_dump(mode="json")
    out["data_folder"] = str(out["data_folder"])
    return out
