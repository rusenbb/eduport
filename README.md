# Eduport

Eduport is a single-device desktop workspace for master's and PhD research and applications. It is a fresh Tauri 2 + Svelte 5 app with SQLite as the source of truth. Markdown is an import, export, and recovery format; files are never used as the live database.

## What works

- Universities, labs, people, programs, applications, publications, projects, mail, documents, and notes. A person has one record and any number of role-bearing links to other records. Labs can link to several institutions. Add your own fields to a record type and use tags and links freely.
- Local SQLite with transactional writes, foreign keys, full-text index, optimistic revisions, recoverable trash, and a consistent backup command.
- Preview and import an old Eduport Markdown vault or an Eduport export. Import merges into the current database only if IDs and files do not conflict. The source folder is left untouched, and legacy originals and attachments are retained. Export writes readable Markdown, a schema/relation manifest, and managed files.
- Assistant with local Ollama or cloud OpenRouter. Choose a provider and model in Settings. Choose the records for each request. Cloud requests require an explicit checkbox for each request. In controlled mode every suggested write waits for review; in delegated mode only the record types you allow may be changed directly. The assistant cannot delete records or send mail.
- Gmail connection using a Google Desktop OAuth client, PKCE, and the read-only Gmail scope. Search, select up to 20 messages, preview, and import them. The original `.eml` is retained for export. Re-importing the same message for one account is blocked by a unique key. Gmail is the first mail provider adapter; the app has no mail sending or background inbox sync.

## Run locally

Requires a Tauri 2 desktop toolchain (Rust, Node.js, and the platform prerequisites).

```sh
npm ci
npm run tauri dev
```

`npm run check`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml` verify the frontend and Rust core. `npm run tauri build` produces a desktop bundle.

The desktop database lives in the platform application-data directory under `io.github.rusenbb.eduport/eduport.sqlite`. Use **Settings → Back up database** before experimenting with imports. Backups are separate SQLite files. **Export Markdown** creates a new `eduport-export-<timestamp>` folder inside the folder you choose.

## Providers

**Ollama:** Install and run Ollama locally, download a model, then select `Ollama` and enter the model name. Eduport calls the local `/api/chat` endpoint. It does not start Ollama or download a model for you.

**OpenRouter:** Select `OpenRouter`, enter a model ID, and save an API key in Settings. The key is stored in the operating system's credential store, outside the SQLite database and Markdown exports. A request to OpenRouter sends your question and only the records you checked. Review the provider's model pricing and privacy terms before using it.

**Gmail:** In Google Cloud, create an OAuth client of type **Desktop app**, configure the Gmail read-only scope for the consent screen, and copy its client ID into Eduport Settings. Google may restrict an unverified app to test users. Sign-in opens your browser and returns to a temporary loopback address on this device. The refresh token stays in the operating system's credential store. Eduport requests read-only access and cannot send or modify messages.

Provider code lives in `src-tauri/src/ai.rs` and `src-tauri/src/mail.rs`; the frontend calls native commands through `src/lib/api.ts`. Additional providers can be added behind these boundaries without changing the record store.

## Import and recovery

The old Eduport Markdown vault can be selected from **Settings → Import Markdown**. The preview lists records, files, warnings, and blockers before any write. An import is one SQLite transaction. Existing records are never overwritten, and a changed source folder invalidates its preview. Legacy Markdown values that cannot be mapped cleanly stay in the preserved original file, and the preview warns about them.

An Eduport Markdown export is round-trippable through the same preview/import flow. A SQLite backup is the fastest complete recovery copy; close Eduport before replacing its database file with a backup. The app currently targets one device. Copying a live SQLite file into Dropbox or iCloud as an active sync mechanism is unsupported.

## Current boundaries

Gmail OAuth and model responses need your own credentials or running local provider; automated tests use local fixtures and do not sign in or spend API credits. This first version supports one connected Gmail account at a time, selected-message import, and local backup/export. Mail attachments are preserved in the original `.eml` and are not yet surfaced as separate document records. There is no scheduled cloud sync, mail sending, or runtime installation of third-party plugins.
