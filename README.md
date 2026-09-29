# Eduport

Eduport is a single-device desktop workspace for master's and PhD research and applications. It is a fresh Tauri 2 + Svelte 5 app with SQLite as the source of truth. Markdown is an import, export, and recovery format; files are never used as the live database.

## What works

- Universities, labs, people, programs, applications, publications, projects, mail, documents, and notes. A person has one record and any number of role-bearing links to other records. Labs can link to several institutions. Each type has relevant fields; program level and application status are required. Add optional or required fields, use tags and links, and drag record types to reorder the sidebar.
- Create custom entity types. Every type has an editable table with inline cells, new rows, draggable and resizable columns, and multiple named views. Each view saves its visible properties, nested AND/OR filter groups, ordered sorts, and grouping. Views are projections over the shared record store, not separate physical SQLite tables.
- Define relationships between entity types first, then link matching records. Eduport includes common academic relationships; custom types can have their own definitions. Links can carry roles and dates.
- Local SQLite with transactional writes, foreign keys, full-text index, optimistic revisions, recoverable trash, and a consistent backup command.
- Preview and import an old Eduport Markdown vault or an Eduport export. Import merges into the current database only if IDs and files do not conflict. The source folder is left untouched, and legacy originals and attachments are retained. Export writes readable Markdown, a schema/relation manifest, and managed files.
- Export or import a single JSON backup with records, schema, relations, views, original files, settings and Gmail import identities. You can include chats, tool events and approval history. JSON imports run against a temporary database copy for preview and apply as one atomic transaction; existing IDs and conflicting settings are never overwritten. Provider keys and Gmail tokens stay in the OS credential store.
- Tool-using assistant with local Ollama or cloud OpenRouter. Chats, messages, model choices, tool steps, and write decisions persist in SQLite; chats can be renamed and archived. The agent can read entity data and imported files through bounded, paginated tools. Cloud requests require consent to share the conversation and any records or files the agent reads for that turn. Writes to chosen entity types can be unrestricted; writes to other types enter a persistent approval queue. Links need permission for both endpoint types. Permanent deletion and mail sending are unavailable to the agent.
- Gmail connection using a Google Desktop OAuth client, PKCE, and the read-only Gmail scope. Search, select up to 20 messages, preview, and import them. The original `.eml` is retained for export. Re-importing the same message for one account is blocked by a unique key. Gmail is the first mail provider adapter; the app has no mail sending or background inbox sync.
- Local MCP server over standard input/output. It exposes the same bounded database read tools as the assistant. Write tools are available only when the server is started with `--allow-writes`; each write still follows Eduport's per-entity permissions or enters the approval queue.

## Run locally

Requires a Tauri 2 desktop toolchain (Rust, Node.js, and the platform prerequisites).

```sh
npm ci
npm run tauri dev
```

For browser-based development, run `npm run dev` and open `http://127.0.0.1:1420/`. This starts a loopback-only Rust API alongside Vite. The browser and desktop app use the **same local SQLite database**, so records and sidebar order survive reloads. Native file dialogs and Gmail sign-in remain desktop-only.

For isolated development, `EDUPORT_DB_PATH`, `EDUPORT_DEV_PORT`, and `EDUPORT_DEV_API_PORT` override the database path and loopback ports.

## MCP server

Build the separate MCP executable with `npm run mcp:build`. On this Mac the result is `src-tauri/target/release/eduport-mcp`. Configure your MCP client to launch that executable over stdio. By default it opens Eduport's local app-data SQLite database and offers read tools only; it never opens a network port. Use `--db /absolute/path/to/eduport.sqlite` or `EDUPORT_DB_PATH` to select another database.

Add `--allow-writes` to the client command only if you want it to propose changes. Eduport's controlled mode queues every write for review; delegated mode applies writes only to entity types you explicitly allowed. This flag does not allow sending email, permanent deletion, credential access, or arbitrary SQL. Open the Assistant page to review pending MCP changes. The macOS `.app` also contains the MCP executable at `Eduport.app/Contents/MacOS/eduport-mcp`.

`npm run check`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml` verify the frontend and Rust core. On macOS, `npm run tauri build -- --bundles app` produces `src-tauri/target/release/bundle/macos/Eduport.app`. The local bundle uses an ad hoc signature. Public distribution requires your own Apple signing and notarization setup.

The desktop database lives in the platform application-data directory under `io.github.rusenbb.eduport/eduport.sqlite`. Use **Settings → Back up database** before experimenting with imports. Backups are separate SQLite files. **Export Markdown** creates a new `eduport-export-<timestamp>` folder inside the folder you choose.

## Providers

**Ollama:** Install and run Ollama locally, download a model with tool support, then select `Ollama` and enter the model name. Eduport calls its local OpenAI-compatible chat endpoint. It does not start Ollama or download a model for you.

**OpenRouter:** Select `OpenRouter`, enter a tool-capable model ID, and save an API key in Settings. The key is stored in the operating system's credential store, outside the SQLite database and Markdown exports. Eduport does not query the credential store at startup; use **Check stored key** if you want to inspect its status. Each cloud turn may send prior messages in that chat plus records and imported files read by tools after consent. Review the provider's model pricing and privacy terms before using it.

Both providers need a model that supports function/tool calls; see the [Ollama tool guide](https://ollama.com/blog/tool-support) and [OpenRouter tool-calling guide](https://openrouter.ai/docs/guides/features/tool-calling).

**Gmail:** In Google Cloud, create an OAuth client of type **Desktop app**, configure the Gmail read-only scope for the consent screen, and copy its client ID into Eduport Settings. Google may restrict an unverified app to test users. Sign-in opens your browser and returns to a temporary loopback address on this device. The refresh token stays in the operating system's credential store. Eduport requests read-only access and cannot send or modify messages.

Provider code lives in `src-tauri/src/ai.rs`, `src-tauri/src/agent.rs`, and `src-tauri/src/mail.rs`; the frontend calls native commands through `src/lib/api.ts`. Additional providers can be added behind these boundaries without changing the record store.

## Import and recovery

The old Eduport Markdown vault can be selected from **Settings → Import Markdown**. The preview lists records, files, warnings, and blockers before any write. An import is one SQLite transaction. Existing records are never overwritten, and a changed source folder invalidates its preview. Legacy Markdown values that cannot be mapped cleanly stay in the preserved original file, and the preview warns about them.

An Eduport Markdown export includes entity types, records, relationship definitions, links, saved views, and managed files, and is round-trippable through the same preview/import flow. It is a readable research-data copy, not an undo system or a complete backup. Use JSON export with **Include assistant chats...** for portable full-history recovery, or SQLite backup for the fastest exact restore. JSON and SQLite do not contain provider passwords or OAuth tokens from the OS credential store. Close Eduport before replacing its database file with a SQLite backup. The app currently targets one device. Copying a live SQLite file into Dropbox or iCloud as an active sync mechanism is unsupported.

## Current boundaries

Gmail OAuth and model responses need your own credentials or running local provider; automated tests use local fixtures and do not sign in or spend API credits. This first version supports one connected Gmail account at a time, selected-message import, and local backup/export. Mail attachments are preserved in the original `.eml` and are not yet surfaced as separate document records. There is no scheduled cloud sync, mail sending, or runtime installation of third-party plugins.
