//! Slug + ID generation: turn a human-friendly entity name into a
//! filename-safe stem, and generate fresh 4-character alphanumeric ids
//! for collision-safe filename suffixes.
//!
//! Output filename convention: `<slug>-<id>.md` (e.g.
//! `stanford-university-7g3k.md`). The slug is decorative — it's the
//! id that guarantees uniqueness on disk.

use std::sync::OnceLock;

const SLUG_MAX_LEN: usize = 60;
const ID_LENGTH: usize = 4;
const ID_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const ID_MAX_RETRIES: usize = 100;

fn non_alnum_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"[^a-z0-9]+").unwrap())
}

/// Folding map for common European Latin characters that don't have an
/// NFKD decomposition (`ø`, `æ`, `ß`, etc.). Anything not here falls
/// through to the regex pass and becomes a hyphen — acceptable v1
/// behaviour for non-Latin scripts.
fn ascii_fold(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            'ø' => out.push('o'),
            'Ø' => out.push('O'),
            'æ' => out.push_str("ae"),
            'Æ' => out.push_str("AE"),
            'œ' => out.push_str("oe"),
            'Œ' => out.push_str("OE"),
            'ß' => out.push_str("ss"),
            'þ' => out.push_str("th"),
            'Þ' => out.push_str("TH"),
            'ð' => out.push('d'),
            'Ð' => out.push('D'),
            'ł' => out.push('l'),
            'Ł' => out.push('L'),
            other => out.push(other),
        }
    }
    out
}

/// Best-effort NFKD decomposition + combining-mark stripping using the
/// `unicode-normalization` property tables. We don't pull `unicode-
/// normalization` in just for this — for the common ASCII-Latin and
/// composed-Latin-with-diacritic cases we strip via a small whitelist.
/// Non-Latin scripts fall through to the regex pass.
///
/// This is a pragmatic v1 — matches the Python implementation's
/// behaviour for European-Latin inputs, which is the dominant case
/// for university/program/lab names. If a non-Latin name produces
/// `untitled`, the user can edit the filename or rename the record.
fn strip_combining_marks(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        // Filter out common combining marks (U+0300..U+036F).
        if (0x0300..=0x036F).contains(&(c as u32)) {
            continue;
        }
        out.push(c);
    }
    out
}

/// Decompose composed Latin characters with diacritics to base + mark,
/// so a subsequent `strip_combining_marks` removes the mark and leaves
/// the base. We use a small lookup table for the most common cases —
/// the same set the Python sidecar covered via NFKD.
fn decompose_latin(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for c in input.chars() {
        let base = match c {
            // Lowercase
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ç' => 'c',
            'ñ' => 'n',
            // Uppercase
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
            'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'Ý' => 'Y',
            'Ç' => 'C',
            'Ñ' => 'N',
            other => {
                out.push(other);
                continue;
            }
        };
        out.push(base);
    }
    out
}

/// Turn a human-friendly name into a filename-safe slug. Lowercase
/// alphanumeric tokens separated by hyphens; leading/trailing hyphens
/// stripped; truncated at a word boundary to 60 characters; falls back
/// to `"untitled"` for empty results.
pub fn generate_slug(name: &str) -> String {
    let folded = decompose_latin(name);
    let folded = strip_combining_marks(&folded);
    let folded = ascii_fold(&folded);
    let folded = folded.to_lowercase();
    let slug = non_alnum_re().replace_all(&folded, "-").to_string();
    let slug = slug.trim_matches('-').to_string();

    let truncated = if slug.len() > SLUG_MAX_LEN {
        let cut = &slug[..SLUG_MAX_LEN];
        // If we sliced through a token, drop the partial.
        if let Some(idx) = cut.rfind('-') {
            cut[..idx].to_string()
        } else {
            cut.to_string()
        }
    } else {
        slug
    };

    if truncated.is_empty() {
        "untitled".into()
    } else {
        truncated
    }
}

/// Generate a fresh 4-character alphanumeric id. Retries on collision
/// via the caller-supplied `exists` predicate. The retry cap is far
/// above any reasonable scenario at expected vault scale (~62^4 = 14.7M
/// slots; collision probability is negligible for vaults under 1M
/// records).
pub fn generate_id<F: FnMut(&str) -> bool>(mut exists: F) -> Option<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..ID_MAX_RETRIES {
        let candidate: String = (0..ID_LENGTH)
            .map(|_| {
                let i = rng.gen_range(0..ID_ALPHABET.len());
                ID_ALPHABET[i] as char
            })
            .collect();
        if !exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic_ascii() {
        assert_eq!(generate_slug("Stanford University"), "stanford-university");
    }

    #[test]
    fn slug_strips_diacritics() {
        assert_eq!(generate_slug("École Polytechnique"), "ecole-polytechnique");
        assert_eq!(generate_slug("Universität München"), "universitat-munchen");
    }

    #[test]
    fn slug_handles_special_latin_chars() {
        assert_eq!(generate_slug("Aarhus Ø"), "aarhus-o");
        assert_eq!(generate_slug("Æthelred"), "aethelred");
    }

    #[test]
    fn slug_truncates_at_word_boundary() {
        let long = "this-is-a-very-very-long-name-that-should-be-truncated-at-a-word-boundary";
        let s = generate_slug(long);
        assert!(s.len() <= SLUG_MAX_LEN);
        // Should not end mid-word.
        assert!(!s.ends_with("-"));
    }

    #[test]
    fn slug_falls_back_to_untitled_on_non_latin_input() {
        // Pure non-Latin produces no alphanumerics → empty → "untitled".
        assert_eq!(generate_slug("中文"), "untitled");
        assert_eq!(generate_slug(""), "untitled");
        assert_eq!(generate_slug("   "), "untitled");
    }

    #[test]
    fn slug_collapses_whitespace_and_punctuation() {
        assert_eq!(generate_slug("Hello,   world!!!"), "hello-world");
        assert_eq!(generate_slug("a/b/c"), "a-b-c");
    }

    #[test]
    fn id_generates_unique_string_when_exists_false() {
        let id = generate_id(|_| false).unwrap();
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn id_returns_none_when_always_exists() {
        let id = generate_id(|_| true);
        assert!(id.is_none());
    }

    #[test]
    fn id_retries_until_unique() {
        // Reject the first id we see, accept the second.
        let mut seen = Vec::new();
        let id = generate_id(|c| {
            if seen.is_empty() {
                seen.push(c.to_string());
                true
            } else {
                false
            }
        });
        assert!(id.is_some());
        assert_ne!(id.unwrap(), seen[0]);
    }
}
