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
  server/                 ← Rust backend
    main.rs               ← Rocket launch and route registration
    db.rs                 ← PostgreSQL pool init and feature script runner
    frontend.rs           ← SPA fallback handler
    models.rs             ← Shared data types (User, Role, Claims, etc.)
    test.rs               ← Backend test module

    shared/               ← Cross-subsystem infrastructure
      mod.rs              ← Module declarations and crate-local re-exports only
      constants.rs        ← STORAGE_ROOT, MUSIC_ROOT, BUILD_ROOT
      crypto.rs           ← Random-token and SHA-256 helpers
      db.rs               ← Database connection and error helpers
      encoding.rs         ← Shared encoding/decoding helpers
      errors.rs           ← ApiError and HTTP error constructors
      files.rs            ← Shared file response, path, and listing helpers
      pagination.rs       ← Reusable page/limit/offset normalization
      tus.rs              ← Shared TUS protocol types and upload workflow

    auth/                 ← Authentication subsystem
      mod.rs              ← Module declarations and crate-local re-exports
      guards.rs           ← AuthenticatedUser guard and DB permission lookup
      helpers.rs          ← Auth-local permission, row, role, and cookie helpers
      jwt.rs              ← Password hashing and JWT/token helpers
      login.rs            ← Login, logout, refresh, and current-user routes
      crud.rs             ← User and role administration
      api_keys.rs         ← User and admin API key management

    files/                ← File storage subsystem
      mod.rs              ← Module declarations and crate-local re-exports
      helpers.rs          ← File-local permission and canonical-path helpers
      list.rs             ← Directory listing
      download.rs         ← File download
      delete.rs           ← File and directory deletion
      folder.rs           ← Folder creation
      rename.rs           ← File and folder rename
      tus.rs              ← File-storage routes over the shared TUS workflow
      upload_links.rs     ← One-time public upload links

    music/                ← Music library subsystem
      mod.rs              ← Module declarations and crate-local re-exports
      crud.rs             ← Global song library routes
      library.rs          ← Personal library routes
      tags.rs             ← Audio metadata scanning and editing
      upload.rs           ← Music routes over shared TUS plus tag scanning

    opensubsonic/         ← OpenSubsonic API
      mod.rs              ← Module declarations and crate-local re-exports
      envelope.rs         ← OpenSubsonic response envelope
      guards.rs           ← SubsonicUser and SubsonicQuery guards
      shared.rs           ← OpenSubsonic-local IDs, ranges, and song helpers
      system.rs           ← Ping, license, and extension endpoints
      browse.rs           ← Browsing, album-list, and genre endpoints
      media.rs            ← Streaming, downloads, cover art, and search
      playlists.rs        ← Playlist endpoints
      starred.rs          ← Star and unstar endpoints
      scrobble_api.rs     ← Scrobbling and now-playing endpoints

  client/                 ← React frontend (TypeScript)
    routes/               ← File-based routing (TanStack Router)
    hooks/                ← Auth, upload, directory, and file operations
    components/           ← UI components
    types/                ← TypeScript type definitions
dbinit/                   ← Idempotent SQL migrations, applied in filename order
storage/
  files/                  ← General file storage root (STORAGE_ROOT)
  music/                  ← Music library root (MUSIC_ROOT)
    .covers/              ← Extracted cover art images
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
- `0007_starred_artist.sql` — relax starred CHECK to allow artist-only stars + unique index

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
- `GET /rest/getMusicFolders`, `getIndexes`, `getMusicDirectory`, `getArtists`, `getArtist`, `getAlbum`, `getSong` — browsing
- `GET /rest/getAlbumList`/`getAlbumList2`, `getGenres` — lists & genres
- `GET /rest/stream`, `download`, `getCoverArt` — media & art
- `GET /rest/search2`/`search3`, `getRandomSongs` — search & random
- `GET /rest/getPlaylists`, `getPlaylist`, `createPlaylist`, `updatePlaylist`, `deletePlaylist` — playlists
- `GET /rest/star`, `unstar`, `getStarred`, `getStarred2` — starred
- `GET /rest/scrobble`, `getNowPlaying` — scrobbling

## Auth Architecture

### Blackfiles Auth (JWT)
- Login sets `access_token` (short-lived JWT cookie, `/` path) and `refresh_token` (HTTP-only, `/api/auth` path)
- `AuthenticatedUser` request guard: reads JWT from cookie, validates, returns user
- `check_permission` is the low-level DB lookup; route code uses `has_permission` for optional checks or `require_permission` for enforcement

### OpenSubsonic Auth
- `SubsonicUser` request guard supports:
  - `apiKey` (recommended) — SHA-256 hashed, stored in `api_keys` table
  - `u`+`p` — argon2 password verification
  - `t`+`s` — token+salt: NOT supported (argon2 is one-way); returns error 41
- All errors returned as `subsonic-response` envelopes with proper error codes (40=wrong password, 41=token not supported, 43=conflicting auth, 44=invalid API key)

## Conventions

### Rust
- Rocket route handlers return `Result<Json<T>, ApiError>` or `Result<TusResponse, ApiError>` where practical
- Cross-subsystem infrastructure belongs in `src/server/shared/`, split by concern and re-exported crate-locally from `shared/mod.rs`
- HTTP error helpers and `ApiError` live in `shared/errors.rs`; database acquisition and error mapping live in `shared/db.rs`
- Shared file/path behavior lives in `shared/files.rs`; shared upload protocol behavior lives in `shared/tus.rs`
- Repeating logic used only by one subsystem belongs in that subsystem's `helpers.rs` or another focused local module
- `get_client(pool)` acquires a database connection and maps pool failures to `ApiError`
- Database scripts are embedded via `include_str!`
- Use `pub(crate)` for shared internal APIs and `pub` only for truly public types
- Keep `mod.rs` files limited to module declarations, imports, and re-exports; implementation belongs in focused files

### TypeScript
- TanStack Router file-based routing under `src/client/routes/`
- Path aliases: `@local/components`, `@local/hooks`, `@local/lib`, `@local/types`
- Auth state via router context (root route's `beforeLoad`)

### Duplication and Shared Code
- Run `bun run duplicates:server`, `bun run duplicates:client`, or `bun run duplicates:all` after structural refactors
- Server detection intentionally requires at least 10 duplicated lines and 100 duplicated tokens, filtering routine Rocket and serde boilerplate
- Treat detector output as an inspection queue, not a mandate to abstract incidental structural similarity
- Extract substantive cross-subsystem duplication into the appropriate `src/server/shared/` concern
- Extract substantive subsystem-only duplication into a focused local helper
- Start with the highest-impact clones, update every callsite, then rerun the relevant duplicate detector
- Run `bun run everything` after duplicate refactors

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
- [x] Phase 4 — OpenSubsonic Browsing
- [x] Phase 5 — OpenSubsonic Media & Search
- [x] Phase 6 — OpenSubsonic Playlists & Annotations
- [x] Phase 6.5 — Refactor
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

## Package Scripts

- `bun run dev` — start the Vite frontend development server
- `bun run build` — build the frontend with Vite
- `bun run fmt` — apply Biome fixes and format Rust with `cargo fmt`
- `bun run lint` — apply Biome fixes and run `cargo clippy`
- `bun run typecheck` — type-check the frontend with TypeScript native preview
- `bun run everything` — run formatting, linting, and frontend type-checking
- `bun run duplicates:client` — scan frontend source for copy-paste duplication
- `bun run duplicates:server` — scan Rust source, requiring 10 lines and 100 tokens per clone
- `bun run duplicates:all` — run both duplicate detectors
- `bun run loc:server` — count and rank Rust source lines by file

## Running

```bash
# Start PostgreSQL (Docker)
docker compose up -d db

# Verify formatting, linting, and frontend types
bun run everything

# Check duplication after structural changes
bun run duplicates:all

# Backend
cargo run

# Frontend development server
bun run dev
```
