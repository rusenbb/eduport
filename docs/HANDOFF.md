# Eduport SQLite snapshot — 29 September 2026

This branch preserves the current single-device SQLite desktop app. It is a checkpoint, not a merge into `origin/main`.

## Current state

- The local app opens and shows 75 records: 13 universities, 13 master's programs, 13 Fall 2027 applications, and 36 `source_note` records imported from `~/Desktop/1416`.
- The 12 selected schools and Penn alternative follow the source notes. The pre-existing UCSD record kept its ID and was enriched; no duplicate UCSD was made. The 36 original Markdown files are stored byte-for-byte in the database, with 52 record links.
- Markdown import/export, previewed JSON import/export, and a consistent SQLite backup command are implemented. JSON can include assistant chats, tool events, and approval history; OS Keychain credentials are excluded. Existing IDs are not overwritten on import.
- The local MCP executable uses stdio. It lists nine read-only tools by default. `--allow-writes` exposes write tools, which still follow Eduport's per-entity approval rules.
- The assistant supports Ollama and OpenRouter; Gmail supports selected-message import. Live model calls and Gmail OAuth were not exercised in this checkpoint.

## Local data snapshots

- SQLite: `~/Library/Application Support/io.github.rusenbb.eduport/backups/eduport-handoff-20260929T085228Z.sqlite`
- Full JSON: `~/Library/Application Support/io.github.rusenbb.eduport/backups/post-1416-import-20260929T083929Z.json`
- An additional pre-import SQLite backup exists in the same directory. Private 1416 content and database backups are not committed to Git.

The SQLite snapshot passed `PRAGMA integrity_check` with 75 records and 36 managed files. The JSON file parsed with 75 records and 36 files; importing it into an empty temporary database produced 75 records, 36 files, and 52 links with an intact database.

## Verification and limits

- `cargo test` (17 tests), `cargo clippy --all-targets -- -D warnings`, `npm run check`, and `npm run build` passed before this checkpoint.
- The macOS `.app` bundle built and reopened. Its UI showed the imported counts, source-note table, custom fields, relationships, and JSON controls. The packaged MCP binary completed a JSON-RPC handshake and listed nine read-only tools.
- The native JSON save dialog could not be completed through macOS UI automation: the automation tool timed out after opening it. The app was restarted and its main window worked. Manually verify the save/open dialog path before calling that UI flow fully tested.
- Imported deadlines, admissions statuses, and outreach notes reflect the dated Markdown sources. They are planning data, not a live check of admissions sites or evidence that an email was sent.

## Next steps

1. Resolve architecture divergence before merging: at this checkpoint `origin/main` had moved 22 commits beyond the common ancestor into a VaultDB-based codebase, while this branch contains seven independent SQLite commits. Review both histories and choose the authoritative product/data model deliberately. Do not fast-forward or overwrite either line.
2. Manually exercise JSON export and import through the native file dialogs, then test a recovery using a disposable database. Keep the current SQLite backup until this is confirmed.
3. Connect the packaged MCP binary to the intended MCP client, verify read tools on the real workspace, then test approval-gated writes against a disposable database before enabling writes on the real one.
4. Validate the 1416 records in the table UI, especially field mapping, links, selected status, and archived source view. Refresh time-sensitive admissions facts from official sources before relying on them.
5. Test Ollama/OpenRouter and Gmail with the user's chosen credentials only when those integrations are needed. No email sending is implemented.
