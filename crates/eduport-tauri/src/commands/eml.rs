//! EML parse command.
//!
//! Wraps `eduport_core::eml::parse_eml`. The frontend sends raw
//! bytes over Tauri's `invoke` channel as a `Vec<u8>` (Tauri
//! handles the binary encoding); we decode and return a typed DTO
//! the frontend already knows about.

use eduport_core::eml::{parse_eml, ParsedEml};
use eduport_core::entity::EmailDirection;
use serde::Serialize;
use tauri::State;

use super::{require_state, CommandError};
use crate::core_state::EduportStateHandle;

#[derive(Debug, Serialize)]
pub struct ParsedEmlDto {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub date: Option<String>,
    pub body: String,
    /// `"inbound"` or `"outbound"`, matching the frontend's
    /// `ParsedEml.direction`.
    pub direction: &'static str,
}

impl From<ParsedEml> for ParsedEmlDto {
    fn from(p: ParsedEml) -> Self {
        Self {
            from: p.from,
            to: p.to,
            cc: p.cc,
            bcc: p.bcc,
            subject: p.subject,
            date: p.date,
            body: p.body,
            direction: match p.direction {
                EmailDirection::Inbound => "inbound",
                EmailDirection::Outbound => "outbound",
            },
        }
    }
}

/// Parse a raw `.eml` byte stream into a structured email body the
/// frontend can preview before deciding to import.
///
/// The user_email used for inbound/outbound discrimination comes
/// from `EduportState` (set at boot time from the persisted
/// settings) — not from a per-call argument, so the frontend can't
/// accidentally flip direction.
#[tauri::command]
pub fn core_parse_eml(
    state: State<'_, EduportStateHandle>,
    bytes: Vec<u8>,
) -> Result<ParsedEmlDto, CommandError> {
    let st = require_state(&state)?;
    let parsed =
        parse_eml(&bytes, &st.user_email).map_err(|e| CommandError::invalid(e.to_string()))?;
    Ok(parsed.into())
}
