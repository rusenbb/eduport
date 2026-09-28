use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use mailparse::MailHeaderMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::db::ExternalMail;

const KEYRING_SERVICE: &str = "io.github.rusenbb.eduport";
const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const MAX_SELECTED: usize = 20;
const MAX_TOTAL_BYTES: usize = 100 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 30 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GmailAccount {
    pub provider: String,
    pub email: String,
    pub client_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GmailMessage {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
    pub snippet: String,
    pub imported: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct MailPreview {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub body: String,
    pub raw_bytes: usize,
    pub imported: bool,
}

#[derive(Debug, Deserialize)]
struct OAuthToken {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailProfile {
    #[serde(rename = "emailAddress")]
    email_address: String,
}

#[derive(Debug, Deserialize)]
struct MessageList {
    #[serde(default)]
    messages: Vec<MessageId>,
}

#[derive(Debug, Deserialize)]
struct MessageId {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GmailRaw {
    raw: String,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
}

pub struct PendingMail {
    pub account: String,
    pub messages: Vec<ExternalMail>,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, "gmail-refresh").map_err(|error| error.to_string())
}

fn pkce_verifier() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub async fn connect(app: &tauri::AppHandle, client_id: &str) -> Result<GmailAccount, String> {
    if client_id.trim().is_empty() || client_id.contains(char::is_whitespace) {
        return Err("Enter a Google OAuth desktop client ID.".into());
    }
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("OAuth callback: {error}"))?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}/",
        listener.local_addr().map_err(|e| e.to_string())?.port()
    );
    let state = Uuid::new_v4().to_string();
    let verifier = pkce_verifier();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut auth = url::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
        .map_err(|error| error.to_string())?;
    auth.query_pairs_mut()
        .append_pair("client_id", client_id.trim())
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", GMAIL_SCOPE)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    app.opener()
        .open_url(auth.as_str(), None::<&str>)
        .map_err(|error| format!("Open Google sign-in: {error}"))?;
    let code = tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
            let mut buffer = [0u8; 8192];
            let len = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
            let request = String::from_utf8_lossy(&buffer[..len]);
            let path = request.lines().next().and_then(|line| line.split_whitespace().nth(1));
            let Some(path) = path else { continue };
            let Ok(callback) = url::Url::parse(&format!("http://localhost{path}")) else { continue };
            let received_state = callback.query_pairs().find(|(key, _)| key == "state").map(|(_, value)| value.to_string());
            if received_state.as_deref() != Some(&state) {
                let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n").await;
                continue;
            }
            let code = callback.query_pairs().find(|(key, _)| key == "code").map(|(_, value)| value.to_string());
            let response = if code.is_some() {
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><title>Eduport</title><p>Gmail connected. You can return to Eduport.</p>"
            } else {
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nGmail authorization was cancelled."
            };
            let _ = stream.write_all(response.as_bytes()).await;
            return code.ok_or_else(|| "Gmail authorization was cancelled.".to_string());
        }
    }).await.map_err(|_| "Google sign-in timed out. Try again.".to_string())??;
    let response = client()?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.trim()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let token: OAuthToken = parse_response(response).await?;
    let refresh = token
        .refresh_token
        .ok_or("Google did not return an offline token. Reconnect and grant access.")?;
    let profile: GmailProfile = parse_response(
        client()?
            .get(format!("{GMAIL_API}/profile"))
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    keyring_entry()?
        .set_password(&refresh)
        .map_err(|error| error.to_string())?;
    Ok(GmailAccount {
        provider: "gmail".into(),
        email: profile.email_address,
        client_id: client_id.trim().into(),
    })
}

async fn access_token(account: &GmailAccount) -> Result<String, String> {
    let refresh = keyring_entry()?
        .get_password()
        .map_err(|_| "Gmail authorization is missing. Connect Gmail again.".to_string())?;
    let response = client()?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", account.client_id.as_str()),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    Ok(parse_response::<OAuthToken>(response).await?.access_token)
}

async fn parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, String> {
    let status = response.status();
    let text = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "Provider returned {status}: {}",
            text.chars().take(500).collect::<String>()
        ));
    }
    serde_json::from_str(&text).map_err(|error| format!("Unexpected provider response: {error}"))
}

pub async fn search(
    account: &GmailAccount,
    query: &str,
    imported: &[String],
) -> Result<Vec<GmailMessage>, String> {
    if query.len() > 1000 {
        return Err("Search query is too long.".into());
    }
    let token = access_token(account).await?;
    let response = client()?
        .get(format!("{GMAIL_API}/messages"))
        .bearer_auth(&token)
        .query(&[("q", query), ("maxResults", "20")])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let list: MessageList = parse_response(response).await?;
    let mut messages = Vec::with_capacity(list.messages.len());
    for item in list.messages {
        let response = client()?
            .get(format!("{GMAIL_API}/messages/{}", item.id))
            .bearer_auth(&token)
            .query(&[("format", "metadata")])
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let metadata: serde_json::Value = parse_response(response).await?;
        let headers = metadata
            .pointer("/payload/headers")
            .and_then(|value| value.as_array());
        let header = |name: &str| {
            headers
                .and_then(|list| {
                    list.iter().find(|entry| {
                        entry
                            .get("name")
                            .and_then(|v| v.as_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case(name))
                    })
                })
                .and_then(|entry| entry.get("value"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string()
        };
        messages.push(GmailMessage {
            id: item.id.clone(),
            subject: header("Subject"),
            from: header("From"),
            date: header("Date"),
            snippet: metadata
                .get("snippet")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            imported: imported.contains(&item.id),
        });
    }
    Ok(messages)
}

pub async fn preview(
    account: &GmailAccount,
    ids: &[String],
    imported: &[String],
) -> Result<(Vec<MailPreview>, PendingMail), String> {
    if ids.is_empty() || ids.len() > MAX_SELECTED {
        return Err("Choose between 1 and 20 messages.".into());
    }
    let mut unique = std::collections::HashSet::new();
    if ids.iter().any(|id| {
        !unique.insert(id) || id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric())
    }) {
        return Err("Invalid or duplicate Gmail message ID.".into());
    }
    let token = access_token(account).await?;
    let mut previews = Vec::with_capacity(ids.len());
    let mut messages = Vec::with_capacity(ids.len());
    let mut total_bytes = 0usize;
    for id in ids {
        let response = client()?
            .get(format!("{GMAIL_API}/messages/{id}"))
            .bearer_auth(&token)
            .query(&[("format", "raw")])
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let payload: GmailRaw = parse_response(response).await?;
        let raw = URL_SAFE_NO_PAD
            .decode(&payload.raw)
            .or_else(|_| URL_SAFE.decode(&payload.raw))
            .or_else(|_| STANDARD.decode(&payload.raw))
            .map_err(|error| format!("Decode Gmail message: {error}"))?;
        total_bytes += raw.len();
        if raw.len() > MAX_MESSAGE_BYTES || total_bytes > MAX_TOTAL_BYTES {
            return Err("Selected mail is too large for one import. Choose fewer messages.".into());
        }
        let (message, display) = parse_eml(id, raw, payload.internal_date.as_deref())?;
        previews.push(MailPreview {
            imported: imported.contains(id),
            ..display
        });
        messages.push(message);
    }
    Ok((
        previews,
        PendingMail {
            account: account.email.clone(),
            messages,
        },
    ))
}

fn parse_eml(
    id: &str,
    raw: Vec<u8>,
    internal_date: Option<&str>,
) -> Result<(ExternalMail, MailPreview), String> {
    let parsed = mailparse::parse_mail(&raw).map_err(|error| format!("Parse email: {error}"))?;
    let subject = parsed
        .headers
        .get_first_value("Subject")
        .unwrap_or_else(|| "(No subject)".into());
    let from = parsed.headers.get_first_value("From").unwrap_or_default();
    let to = parsed.headers.get_first_value("To").unwrap_or_default();
    let header_date = parsed.headers.get_first_value("Date").unwrap_or_default();
    let date = chrono::DateTime::parse_from_rfc2822(&header_date)
        .ok()
        .map(|value| value.format("%Y-%m-%d").to_string())
        .or_else(|| {
            internal_date
                .and_then(|value| value.parse::<i64>().ok())
                .and_then(DateTime::<Utc>::from_timestamp_millis)
                .map(|value| value.format("%Y-%m-%d").to_string())
        })
        .unwrap_or_default();
    let body = plain_text(&parsed).unwrap_or_else(|| {
        "[No plain-text body. Original message is preserved in the Markdown export.]".into()
    });
    let body = body.chars().take(100_000).collect::<String>();
    let display = MailPreview {
        id: id.into(),
        subject: subject.clone(),
        from: from.clone(),
        to: to.clone(),
        date: date.clone(),
        body: body.chars().take(800).collect(),
        raw_bytes: raw.len(),
        imported: false,
    };
    Ok((
        ExternalMail {
            external_id: id.into(),
            title: subject,
            body,
            from,
            to,
            date,
            raw,
        },
        display,
    ))
}

fn plain_text(part: &mailparse::ParsedMail<'_>) -> Option<String> {
    if part.ctype.mimetype.eq_ignore_ascii_case("text/plain") {
        return part.get_body().ok();
    }
    for child in &part.subparts {
        if let Some(text) = plain_text(child) {
            return Some(text);
        }
    }
    None
}

pub fn take_pending(
    pending: &Mutex<Option<PendingMail>>,
    ids: &[String],
) -> Result<PendingMail, String> {
    let mut guard = pending
        .lock()
        .map_err(|_| "mail import lock poisoned".to_string())?;
    let plan = guard
        .as_ref()
        .ok_or("Preview selected messages before importing.")?;
    if plan
        .messages
        .iter()
        .map(|item| &item.external_id)
        .collect::<Vec<_>>()
        != ids.iter().collect::<Vec<_>>()
    {
        return Err("Selection changed. Preview messages again before importing.".into());
    }
    guard.take().ok_or("No pending messages.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_mail_and_keep_original() {
        let raw = b"From: A <a@example.com>\r\nTo: B <b@example.com>\r\nSubject: Research call\r\nDate: Mon, 28 Sep 2026 12:00:00 +0000\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nHello from a lab.\r\n".to_vec();
        let (mail, preview) = parse_eml("abc123", raw.clone(), None).unwrap();
        assert_eq!(mail.body, "Hello from a lab.\r\n");
        assert_eq!(mail.raw, raw);
        assert_eq!(preview.date, "2026-09-28");
    }

    #[test]
    fn pending_import_requires_exact_preview_selection() {
        let pending = Mutex::new(Some(PendingMail {
            account: "x@y.test".into(),
            messages: vec![ExternalMail {
                external_id: "one".into(),
                title: "One".into(),
                body: String::new(),
                from: String::new(),
                to: String::new(),
                date: String::new(),
                raw: vec![],
            }],
        }));
        assert!(take_pending(&pending, &["two".into()]).is_err());
        assert!(take_pending(&pending, &["one".into()]).is_ok());
    }
}
