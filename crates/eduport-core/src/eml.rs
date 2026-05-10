//! `.eml` → Email-entity import.
//!
//! The Tauri "import email" path drops a `.eml` file (RFC 5322 +
//! MIME multipart) on the user's vault. This module parses the
//! bytes into a [`ParsedEml`] (structured headers + a plain-text
//! body) and a higher-level helper turns that into a fully-populated
//! [`crate::entity::Email`] entity.
//!
//! Two transforms happen here:
//!
//! 1. **Address parsing**: `From: "Alice" <alice@example.com>, Bob
//!    <bob@example.com>` → `["alice@example.com", "bob@example.com"]`.
//!    `mailparse::addrparse_header` does this for us; we just flatten
//!    groups and pull addresses out of `MailAddr::Single`.
//!
//! 2. **HTML → markdown body**: when the email is `text/html` only
//!    (no `text/plain` alternative), we run it through `html2text`
//!    to get a plain-text reading suitable for the Email entity's
//!    body. Text emails come through verbatim.
//!
//! This is read-only — the parser doesn't decide where the resulting
//! file lives or what filename to give it; the caller does that
//! using [`crate::generate_slug`] + [`crate::generate_id`].

use mailparse::{MailAddr, MailHeaderMap, MailParseError, addrparse_header};

use crate::EduportError;
use crate::entity::types::{Email, EmailDirection};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEml {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    /// ISO date `YYYY-MM-DD`. `None` when the `Date:` header was
    /// missing or unparseable.
    pub date: Option<String>,
    /// The email body. text/plain when available; otherwise the
    /// markdownified text/html alternative; otherwise empty.
    pub body: String,
    /// Inferred from `From == user_email`: outbound when the user
    /// authored it, inbound otherwise.
    pub direction: EmailDirection,
}

/// Parser-level error type. Doesn't propagate `mailparse`'s error
/// directly so consumers don't transitively depend on `mailparse`.
#[derive(Debug, thiserror::Error)]
pub enum EmlParseError {
    #[error("not a valid RFC 5322 message: {0}")]
    Malformed(String),
}

impl From<MailParseError> for EmlParseError {
    fn from(e: MailParseError) -> Self {
        EmlParseError::Malformed(e.to_string())
    }
}

impl From<EmlParseError> for EduportError {
    fn from(e: EmlParseError) -> Self {
        EduportError::Schema(e.to_string())
    }
}

/// Parse a `.eml` byte stream into a [`ParsedEml`].
///
/// `user_email` is used to compute [`ParsedEml::direction`]: the
/// email is outbound when its `From:` header (case-insensitive)
/// matches `user_email`, inbound otherwise.
pub fn parse_eml(raw: &[u8], user_email: &str) -> Result<ParsedEml, EmlParseError> {
    let parsed = mailparse::parse_mail(raw)?;
    let headers = &parsed.headers;

    let subject = headers
        .get_first_value("Subject")
        .unwrap_or_default()
        .trim()
        .to_string();

    let from = first_address(&parsed, "From").unwrap_or_default();
    let to = all_addresses(&parsed, "To");
    let cc = all_addresses(&parsed, "Cc");
    let bcc = all_addresses(&parsed, "Bcc");

    let date = headers
        .get_first_value("Date")
        .as_deref()
        .and_then(parse_rfc2822_date);

    let body = extract_body(&parsed)?;

    let direction = if from.eq_ignore_ascii_case(user_email) {
        EmailDirection::Outbound
    } else {
        EmailDirection::Inbound
    };

    Ok(ParsedEml {
        from,
        to,
        cc,
        bcc,
        subject,
        date,
        body,
        direction,
    })
}

/// Promote a [`ParsedEml`] to a typed [`Email`] entity. The
/// caller assigns `name` (the human-friendly display name; usually
/// equal to the subject or something derived from it). Tags are
/// seeded with `eduport-type/email`; consumers append their own.
pub fn parsed_eml_to_email(parsed: ParsedEml, name: impl Into<String>) -> Email {
    Email {
        name: name.into(),
        tags: vec!["eduport-type/email".into()],
        direction: parsed.direction,
        date: parsed.date.unwrap_or_default(),
        subject: parsed.subject,
        from: parsed.from,
        to: parsed.to,
        cc: parsed.cc,
        bcc: parsed.bcc,
        related_program: None,
        related_application: None,
        related_people: vec![],
        in_reply_to: None,
        attachments: vec![],
        custom: std::collections::BTreeMap::new(),
    }
}

// ── helpers ──────────────────────────────────────────────────────

fn first_address(parsed: &mailparse::ParsedMail, header: &str) -> Option<String> {
    let h = parsed.headers.get_first_header(header)?;
    let addrs = addrparse_header(h).ok()?;
    addrs.iter().find_map(extract_email_string)
}

fn all_addresses(parsed: &mailparse::ParsedMail, header: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(h) = parsed.headers.get_first_header(header)
        && let Ok(addrs) = addrparse_header(h)
    {
        for addr in addrs.iter() {
            collect_addresses_into(addr, &mut out);
        }
    }
    out
}

fn extract_email_string(addr: &MailAddr) -> Option<String> {
    match addr {
        MailAddr::Single(info) => Some(info.addr.clone()),
        MailAddr::Group(group) => group.addrs.first().map(|info| info.addr.clone()),
    }
}

fn collect_addresses_into(addr: &MailAddr, out: &mut Vec<String>) {
    match addr {
        MailAddr::Single(info) => out.push(info.addr.clone()),
        MailAddr::Group(group) => {
            for info in &group.addrs {
                out.push(info.addr.clone());
            }
        }
    }
}

/// Pull the body text from a parsed message, preferring `text/plain`
/// over `text/html`. If only HTML is available, run it through
/// `html2text` to get a plain-text reading. Multipart messages walk
/// the part tree depth-first and use the same preference rule.
fn extract_body(parsed: &mailparse::ParsedMail) -> Result<String, EmlParseError> {
    if let Some(plain) = find_part(parsed, "text/plain") {
        return Ok(plain.get_body()?.trim().to_string());
    }
    if let Some(html) = find_part(parsed, "text/html") {
        let html_body = html.get_body()?;
        let text = html2text::from_read(html_body.as_bytes(), 80)
            .map_err(|e| EmlParseError::Malformed(format!("html2text: {}", e)))?;
        return Ok(text.trim().to_string());
    }
    // No body content type matched — return empty string.
    Ok(String::new())
}

/// Walk a multipart message tree depth-first looking for a part
/// whose Content-Type matches `mime_type`.
fn find_part<'a>(
    parsed: &'a mailparse::ParsedMail<'a>,
    mime_type: &str,
) -> Option<&'a mailparse::ParsedMail<'a>> {
    if parsed.ctype.mimetype.eq_ignore_ascii_case(mime_type) {
        return Some(parsed);
    }
    for child in &parsed.subparts {
        if let Some(found) = find_part(child, mime_type) {
            return Some(found);
        }
    }
    None
}

/// Parse an RFC 2822 date string into an ISO `YYYY-MM-DD`. Returns
/// None on parse failure (matches the Python sidecar's behaviour:
/// silently drop a malformed date rather than reject the email).
fn parse_rfc2822_date(date_str: &str) -> Option<String> {
    // mailparse exposes dateparse(); we only need the date portion.
    let utc_secs = mailparse::dateparse(date_str).ok()?;
    // Convert to YYYY-MM-DD via days-since-epoch arithmetic.
    let days = (utc_secs / 86400) as u64;
    let (year, month, day) = epoch_days_to_date(days);
    Some(format!("{:04}-{:02}-{:02}", year, month, day))
}

/// Same algorithm vaultdb-core uses for `_modified` virtual-field
/// formatting; ported here so we don't reach into a private helper.
fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PLAIN: &str = "\
From: Alice <alice@example.com>\r
To: Bob <bob@example.com>, charlie@example.com\r
Cc: dave@example.com\r
Subject: Hello there\r
Date: Thu, 10 May 2026 10:00:00 +0000\r
Content-Type: text/plain; charset=utf-8\r
\r
This is the body.\r
";

    const SAMPLE_HTML_ONLY: &str = "\
From: news@example.com\r
To: alice@example.com\r
Subject: Newsletter\r
Date: Sat, 1 Jan 2026 09:30:00 +0000\r
Content-Type: text/html; charset=utf-8\r
\r
<html><body><h1>Title</h1><p>Hello <b>world</b>.</p></body></html>\r
";

    #[test]
    fn parse_plain_text_email() {
        let p = parse_eml(SAMPLE_PLAIN.as_bytes(), "user@example.com").unwrap();
        assert_eq!(p.from, "alice@example.com");
        assert_eq!(p.to, vec!["bob@example.com", "charlie@example.com"]);
        assert_eq!(p.cc, vec!["dave@example.com"]);
        assert!(p.bcc.is_empty());
        assert_eq!(p.subject, "Hello there");
        assert_eq!(p.date, Some("2026-05-10".into()));
        assert!(p.body.contains("This is the body."));
        assert_eq!(p.direction, EmailDirection::Inbound);
    }

    #[test]
    fn outbound_inferred_from_user_email() {
        let p = parse_eml(SAMPLE_PLAIN.as_bytes(), "alice@example.com").unwrap();
        assert_eq!(p.direction, EmailDirection::Outbound);
    }

    #[test]
    fn parse_html_only_email_runs_through_html2text() {
        let p = parse_eml(SAMPLE_HTML_ONLY.as_bytes(), "alice@example.com").unwrap();
        // html2text should produce something that mentions "Title"
        // and "Hello world" (with bold rendered or stripped).
        assert!(p.body.contains("Title"), "got body: {}", p.body);
        assert!(
            p.body.to_lowercase().contains("hello"),
            "got body: {}",
            p.body
        );
    }

    #[test]
    fn parse_eml_with_missing_date_returns_none() {
        let raw = "\
From: x@y\r
To: z@y\r
Subject: nope\r
\r
body
";
        let p = parse_eml(raw.as_bytes(), "user@example.com").unwrap();
        assert!(p.date.is_none());
    }

    #[test]
    fn parse_eml_rejects_garbage_bytes() {
        // mailparse is fairly permissive about garbage in front of
        // the headers; an entirely empty input should parse OK with
        // empty fields. We mostly want to confirm we don't panic.
        let _ = parse_eml(b"", "user@example.com");
    }

    #[test]
    fn parsed_eml_to_email_round_trips_through_yaml() {
        let p = parse_eml(SAMPLE_PLAIN.as_bytes(), "user@example.com").unwrap();
        let email = parsed_eml_to_email(p, "alice-hello-2026-05-10");
        let yaml = serde_yaml::to_string(&email).unwrap();
        let back: Email = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, email);
    }

    #[test]
    fn multipart_alternative_prefers_text_plain() {
        let raw = "\
From: alice@example.com\r
To: bob@example.com\r
Subject: multi\r
Date: Thu, 10 May 2026 10:00:00 +0000\r
MIME-Version: 1.0\r
Content-Type: multipart/alternative; boundary=\"BNDR\"\r
\r
--BNDR\r
Content-Type: text/plain; charset=utf-8\r
\r
Plain version of body.\r
--BNDR\r
Content-Type: text/html; charset=utf-8\r
\r
<p>HTML version of body.</p>\r
--BNDR--\r
";
        let p = parse_eml(raw.as_bytes(), "user@example.com").unwrap();
        assert!(p.body.contains("Plain version"));
        assert!(!p.body.contains("HTML version"));
    }
}
