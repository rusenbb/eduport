from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state
from eduport.index.reconcile import reconcile
from eduport.settings import Settings, save_settings
from eduport.store.files import EntityFileStore
from eduport.store.schema_store import SchemaStore
from eduport.store.trash import LocalTrash

router = APIRouter()


@router.get("/settings")
def get_settings(state: AppState = Depends(get_state)) -> dict:
    payload = state.settings.model_dump(mode="json")
    payload["data_folder"] = str(payload["data_folder"])
    return payload


@router.put("/settings")
def put_settings(payload: Settings, state: AppState = Depends(get_state)) -> dict:
    data_folder_changed = payload.data_folder != state.settings.data_folder
    payload.data_folder.mkdir(parents=True, exist_ok=True)
    payload.resolved_attachments_folder().mkdir(parents=True, exist_ok=True)
    payload.resolved_notes_folder().mkdir(parents=True, exist_ok=True)
    if state.settings_path is not None:
        save_settings(payload, state.settings_path)
    state.settings = payload
    if data_folder_changed:
        state.file_store = EntityFileStore(payload.data_folder)
        state.trash = LocalTrash(payload.data_folder)
        state.schema_store = SchemaStore(payload.data_folder)
        state.schema_store.load()
        reconcile(state.conn, payload.data_folder, schema=state.schema_store.current())
    out = payload.model_dump(mode="json")
    out["data_folder"] = str(out["data_folder"])
    return out
