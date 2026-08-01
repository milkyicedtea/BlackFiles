# Blackfiles — Agent Guide

## Overview

Blackfiles is a self-hosted file storage and music library server. It has two main subsystems:

- **File storage** — general-purpose file upload/download/browse with role-based permissions
- **Music library** — audio file management with an OpenSubsonic API for external clients

## Stack

| Layer      | Technology                                              |
|------------|---------------------------------------------------------|
| Backend    | Rust (Rocket 0.5.1)                                     |
| Database   | PostgreSQL (deadpool-postgres, tokio-postgres)          |
| Frontend   | React (TanStack Router, Mantine UI)                     |
| Auth       | JWT (access + refresh cookies), argon2 password hashing |
| Uploads    | TUS resumable upload protocol                           |
| Music tags | `lofty` 0.24 (MP3, FLAC, M4A, OGG, WAV)                 |
| Build      | Vite (frontend), Cargo (backend)                        |

## Project Layout

```
src/
  server/           ← Rust backend
    main.rs         ← Rocket launch, route registration
    auth.rs         ← Login/logout, JWT, user/role CRUD
    guards.rs       ← AuthenticatedUser request guard, check_permission
    models.rs       ← Shared data types (User, Role, Claims, etc.)
    shared.rs       ← Constants (STORAGE_ROOT, MUSIC_ROOT), error helpers, FileEntry
    db.rs           ← PostgreSQL pool init, feature script runner
    files.rs        ← Download, delete, create folder, rename (file storage)
    list.rs         ← Directory listing (file storage)
    tus.rs          ← TUS resumable uploads (file storage)
    upload_links.rs ← One-time public upload links
    frontend.rs     ← SPA fallback handler
    # Music (new)
    music.rs        ← Tag scanner, song CRUD, personal library
    music_upload.rs ← TUS uploads for music (post-upload tag scanning)
    api_keys.rs     ← API key management (user + admin), ApiKeyUser guard
    subsonic.rs     ← OpenSubsonic API: response envelope, SubsonicUser guard, system endpoints
  client/            ← React frontend (TypeScript)
    routes/          ← File-based routing (TanStack Router)
    hooks/           ← Auth, upload, directory, file operations
    components/      ← UI components
    types/           ← TypeScript type definitions
dbinit/              ← Idempotent SQL migration scripts (run on startup in order)
storage/             ← Runtime data
  files/             ← General file storage root (STORAGE_ROOT)
  music/             ← Music library root (MUSIC_ROOT)
    .covers/         ← Extracted cover art images
```

## Constants

- `STORAGE_ROOT = "storage/files"` — general file storage
- `MUSIC_ROOT = "storage/music"` — music library storage
- `BUILD_ROOT = "dist"` — frontend build output

## Database Migrations

Scripts in `dbinit/` are applied idempotently in lexicographic order on startup via `DatabaseFeatures` fairing. All use `CREATE TABLE IF NOT EXISTS` / `INSERT ... ON CONFLICT DO NOTHING`.

Current migrations:
- `0000_init.sql` — users, roles, permissions, sessions
- `0001_dense_role_positions.sql` — role ordering function
- `0002_core_seed.sql` — default roles, permissions
- `0003_upload_links.sql` — one-time upload links
- `0004_upload_sessions.sql` — TUS upload sessions
- `0005_public_upload_sessions.sql` — public TUS support
- `0006_music_library.sql` — songs, user_songs, cover_art, api_keys, playlists, starred, scrobbles

## API Route Structure

### File Storage (`/api/`)
- `GET/POST /api/auth/*` — authentication
- `GET/POST/PUT/DELETE /api/users/*` — user management
- `GET/POST/PUT/DELETE /api/roles/*` — role management
- `GET /api/list/*` — directory listing
- `GET /api/files/*` — file download
- `DELETE /api/files/*` — file/directory deletion
- `POST /api/folders` — create folder
- `PUT /api/rename` — rename file/folder
- TUS endpoints at `/api/uploads/*` and `/api/public/upload-links/*`
- `GET/POST/DELETE /api/upload-links` — upload link management

### Music (`/api/music/`)
- `GET/POST /api/music/songs` — global library
- `DELETE /api/music/songs/<id>` — delete song
- `PUT /api/music/songs/<id>/tags` — edit tags
- `POST /api/music/scan` — re-scan file tags
- `GET /api/music/library` — personal library
- `POST/DELETE /api/music/library/<song_id>` — add/remove from personal
- TUS endpoints at `/api/music/uploads/*`
- `GET/POST/DELETE /api/music/api-keys` — personal API key management
- `GET/DELETE /api/admin/api-keys` — admin API key management

### OpenSubsonic (`/rest/`)
- `GET /rest/ping` — health check
- `GET /rest/getLicense` — license
- `GET /rest/getOpenSubsonicExtensions` — capabilities
- (More endpoints coming in Phase 4+)

## Auth Architecture

### Blackfiles Auth (JWT)
- Login sets `access_token` (short-lived JWT cookie, `/` path) and `refresh_token` (HTTP-only, `/api/auth` path)
- `AuthenticatedUser` request guard: reads JWT from cookie, validates, returns user
- `check_permission(pool, user_id, permission)` — DB-backed permission check

### OpenSubsonic Auth
- `SubsonicUser` request guard supports:
  - `apiKey` (recommended) — SHA-256 hashed, stored in `api_keys` table
  - `u`+`p` — argon2 password verification
  - `t`+`s` — token+salt: NOT supported (argon2 is one-way); returns error 41
- All errors returned as `subsonic-response` envelopes with proper error codes (40=wrong password, 41=token not supported, 43=conflicting auth, 44=invalid API key)

## Conventions

### Rust
- Rocket route handlers return `Result<Json<T>, (Status, Json<serde_json::Value>)>` or `Result<TusResponse, ApiError>`
- Error helpers in `shared.rs`: `bad_request`, `not_found`, `forbidden`, `conflict`, `server_error`, `db_error`
- `get_client(pool)` acquires a DB connection from the pool
- Database scripts are embedded via `include_str!`
- Use `pub(crate)` for shared internal APIs, `pub` only for truly public types

### TypeScript
- TanStack Router file-based routing under `src/client/routes/`
- Path aliases: `@local/components`, `@local/hooks`, `@local/lib`, `@local/types`
- Auth state via router context (root route's `beforeLoad`)

## Key Dependencies

### Rust
- `rocket` 0.5.1 — web framework
- `deadpool-postgres` + `tokio-postgres` — async PostgreSQL
- `argon2` — password hashing (NOT reversible; can't support Subsonic `t`+`s` auth)
- `jsonwebtoken` — JWT
- `lofty` 0.24 — audio metadata (ID3, Vorbis, MP4 tags)
- `sha2` — SHA-256 for API key hashing
- `uuid` — UUID generation

### Frontend
- `@mantine/core` — UI components
- `@tanstack/react-router` — routing
- `mantine-datatable` — data tables
- `@tabler/icons-react` — icons

## Roadmap
See `ROADMAP_MUSIC.md` for the music library implementation plan. Current status:

- [x] Phase 0 — Foundation (DB schema, storage roots, migrations)
- [x] Phase 1 — Upload & Tag Scanning
- [x] Phase 2 — API Keys
- [x] Phase 3 — OpenSubsonic Scaffolding
- [ ] Phase 4 — OpenSubsonic Browsing
- [ ] Phase 5 — OpenSubsonic Media & Search
- [ ] Phase 6 — OpenSubsonic Playlists & Annotations
- [ ] Phase 7 — Blackfiles Music UI
- [ ] Phase 8 — Polish & Tier 2/3 Endpoints

## Agent Rules

### Phase Completion
- After finishing every phase, run `bun run everything` and fix any failures before marking the phase done.

### File Editing
- **NEVER** use `sed`, `awk`, `cat`, redirections (`>`, `>>`), or other shell commands to modify source files.
- Use the agentic tools: `edit` (surgical patches), `write` (create/overwrite), `read` (read-only).
- Use agent tasks if available; otherwise, use ROADMAP_<feature>.md files to keep track of progress.
- **NEVER** read external library source files (e.g., under `~/.cargo/registry/`) unless explicitly asked or absolutely necessary to resolve a compilation error.

### Code Intelligence
- **MANDATORY**: Use `lsp` for cross-file operations (renames, references, definitions, code actions) whenever a language server is available.
- Prefer `lsp rename` over text-based renames — text tools miss shadowed imports, re-exports, and cross-file callsites.

### Search
- Use `grep` (not `bash grep`/`rg`) for regex search.
- Use `glob` (not `ls`/`find`) for file discovery.

## Running

```bash
# Start PostgreSQL (docker)
docker compose up -d db

# Verify everything (format + lint + typecheck)
bun run everything

# Backend
cargo run

# Frontend dev server
bun run dev
```
