# Music Library Roadmap

## Overview

A music library system separate from general file storage. Backed by a full OpenSubsonic API implementation so external clients (DSub, Symfonium, Tempo, etc.) can browse and stream. Blackfiles itself provides no playback — only library management (upload, tag editing, delete) and the bridge between global and personal libraries.

## Library Model

| Library      | Scope     | Access                               |
|--------------|-----------|--------------------------------------|
| **Global**   | All users | Blackfiles UI only                   |
| **Personal** | Per-user  | Blackfiles UI + OpenSubsonic clients |

- Users upload to the global library (single source of truth; duplicates are easier to spot).
- Users "add" songs from global to their personal library (DB reference — no file copy).
- External clients see only the authenticated user's personal library via OpenSubsonic.
- Deleting from personal = removing the reference. Deleting from global = removing the file + cascading all personal references.

## Storage Layout

```
storage/
  files/          ← general file storage (was storage/; migration needed)
  music/          ← music library root
    {artist}/{album}/{track}.{ext}   ← audio files (or flat UUID if preferred)
    .covers/      ← extracted cover art images (dot-prefix: hidden from directory listings)
```

`STORAGE_ROOT` moves to `storage/files/`. New constant `MUSIC_ROOT = "storage/music/"`.

## Database Schema

```sql
-- Songs (one row per audio file)
CREATE TABLE songs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_path TEXT NOT NULL UNIQUE,          -- relative to storage/music/
    title TEXT NOT NULL,
    artist TEXT NOT NULL DEFAULT 'Unknown',
    album TEXT NOT NULL DEFAULT 'Unknown',
    album_artist TEXT,
    genre TEXT,
    year SMALLINT,
    track_number SMALLINT,
    disc_number SMALLINT DEFAULT 1,
    duration_seconds REAL,
    size_bytes BIGINT NOT NULL,
    format TEXT,                             -- mp3, flac, aac, ogg, etc.
    bitrate_kbps SMALLINT,
    has_cover_art BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_songs_artist ON songs(artist);
CREATE INDEX idx_songs_album ON songs(album);
CREATE INDEX idx_songs_genre ON songs(genre);

-- Personal library (junction)
CREATE TABLE user_songs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, song_id)
);

-- Cover art (extracted from tags or uploaded)
CREATE TABLE cover_art (
    song_id UUID PRIMARY KEY REFERENCES songs(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,                 -- relative to storage/music/.covers/
    mime_type TEXT NOT NULL DEFAULT 'image/jpeg',
    width SMALLINT,
    height SMALLINT
);

-- API keys for OpenSubsonic clients
-- The raw key is shown once on creation, then stored as SHA-256 hash.
-- Admins can list/revoke any user's keys, but never see the raw key values.
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,           -- SHA-256 of the generated key
    label TEXT,                              -- user-given name ("My Phone", "DSub")
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Playlists
CREATE TABLE playlists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    comment TEXT,
    public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE playlist_songs (
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, song_id)
);

-- Starred
CREATE TABLE starred (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    song_id UUID REFERENCES songs(id) ON DELETE CASCADE,
    album_name TEXT,
    artist_name TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT starred_exactly_one_target CHECK (
        (song_id IS NOT NULL AND album_name IS NULL AND artist_name IS NULL) OR
        (song_id IS NULL AND album_name IS NOT NULL AND artist_name IS NOT NULL)
    ),
    UNIQUE (user_id, song_id),
    UNIQUE (user_id, artist_name, album_name)
);

-- Scrobbles (play history)
CREATE TABLE scrobbles (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    played_at TIMESTAMPTZ NOT NULL,
    submission BOOLEAN DEFAULT TRUE          -- true = client submitted, false = now-playing ping
);
```

## Permissions

| `music_upload` | Music | Upload files to the global library |
| `music_delete` | Music | Delete songs from the global library |
| `music_edit_tags` | Music | Edit song metadata (ID3 tags + DB) |
| `music_manage_api_keys` | Music | Create/revoke own OpenSubsonic API keys |
| `music_manage_all_api_keys` | Music | Admin: list/revoke any user's API keys |

`music_add_to_library` (adding from global to personal) is granted to all authenticated users — no separate permission needed.

## Auth Bridge: Blackfiles → OpenSubsonic

```
┌──────────────┐     apiKey      ┌──────────────┐
│  Subsonic    │ ──────────────► │  /rest/*     │
│  Client      │                 │  Endpoints   │
│  (DSub, etc) │                 │              │
└──────────────┘                 │  Lookup key  │
                                 │  → user_id   │
                                 │  → personal  │
                                 │    library   │
                                 └──────────────┘
```

- User generates an API key in Blackfiles Settings → stored hashed in `api_keys`.
- Subsonic client configured with server URL `https://blackfiles.example.com/rest` and the API key as `apiKey` parameter.
- `/rest/` auth middleware: extract `apiKey` from query params, SHA-256 it, look up in `api_keys`, attach user.
- Also support legacy `u`+`p` (password) and `t`+`s` (token+salt MD5) for compatibility, using the same `users` table credentials.
- Key revocation: delete the row. Immediate effect.

## API Routes

### Blackfiles Music API (`/api/music/`)

| Method   | Path                          | Description                                                                           |
|----------|-------------------------------|---------------------------------------------------------------------------------------|
| `GET`    | `/api/music/songs`            | List global library (search, paginate, sort)                                          |
| `POST`   | `/api/music/songs`            | Upload one or more audio files                                                        |
| `DELETE` | `/api/music/songs/<id>`       | Delete a song from global                                                             |
| `PUT`    | `/api/music/songs/<id>/tags`  | Edit tags (writes DB + ID3 tags)                                                      |
| `GET`    | `/api/music/library`          | List user's personal library                                                          |
| `POST`   | `/api/music/library/<songId>` | Add song from global to personal                                                      |
| `GET`    | `/api/music/api-keys`         | List user's own API keys                                                              |
| `POST`   | `/api/music/api-keys`         | Generate new API key                                                                  |
| `DELETE` | `/api/music/api-keys/<id>`    | Revoke an API key                                                                     |
| `GET`    | `/api/admin/api-keys`         | Admin: list all users' API keys (label, user, created, last used — never the raw key) |
| `DELETE` | `/api/admin/api-keys/<id>`    | Admin: revoke any user's API key                                                      |

### OpenSubsonic API (`/rest/`)

All under `/rest/` with mandatory params `u`/`p` or `t`/`s` or `apiKey`, plus `v`, `c`, `f`. Returns `subsonic-response` envelope.

#### Phase A — Essential (client won't work without these)

| Endpoint                         | Notes                                                                   |
|----------------------------------|-------------------------------------------------------------------------|
| `ping`                           | Health check                                                            |
| `getLicense`                     | Return valid license payload                                            |
| `getOpenSubsonicExtensions`      | Declare supported extensions                                            |
| `getMusicFolders`                | Return single folder "Personal Library"                                 |
| `getIndexes`                     | Alphabetical artist index (ID3 mode)                                    |
| `getMusicDirectory`              | File-structure browse (file mode)                                       |
| `getArtists`                     | All artists with album counts                                           |
| `getArtist`                      | Single artist + their albums                                            |
| `getAlbum`                       | Single album + track list                                               |
| `getSong`                        | Single song details                                                     |
| `getAlbumList` / `getAlbumList2` | Paginated album lists (newest, random, alphabetical, by genre, by year) |
| `getGenres`                      | All genres with song/album counts                                       |
| `stream`                         | Stream audio file (Range support)                                       |
| `download`                       | Full file download                                                      |
| `getCoverArt`                    | Serve cover art by song ID                                              |
| `search2` / `search3`            | Search artists/albums/songs                                             |
| `getRandomSongs`                 | Random song selection                                                   |
| `getPlaylists`                   | List user's playlists                                                   |
| `getPlaylist`                    | Single playlist with entries                                            |
| `createPlaylist`                 | Create + optionally add song IDs                                        |
| `updatePlaylist`                 | Rename, update comment, update entries                                  |
| `deletePlaylist`                 | Delete playlist                                                         |
| `star` / `unstar`                | Star/unstar songs, albums, artists                                      |
| `scrobble`                       | Record play (with optional `submission` flag)                           |
| `getStarred` / `getStarred2`     | List starred items                                                      |

#### Phase B — Important for UX

| Endpoint                               | Notes                                                   |
|----------------------------------------|---------------------------------------------------------|
| `getArtistInfo` / `getArtistInfo2`     | Can return empty bio/images initially                   |
| `getAlbumInfo` / `getAlbumInfo2`       | Can return empty notes/images initially                 |
| `getSimilarSongs` / `getSimilarSongs2` | Genre-based similarity; can be basic                    |
| `getTopSongs`                          | Most played by user                                     |
| `getSongsByGenre`                      | Filter songs by genre                                   |
| `getNowPlaying`                        | Currently playing (track from scrobbles)                |
| `getAvatar`                            | User avatar (can return 404)                            |
| `getUser` / `getUsers`                 | User management — read-only mapping to Blackfiles users |

#### Phase C — Stub responses (return empty/unsupported)

`getVideos`, `getVideoInfo`, `getCaptions`, `hls`, `getShares`, `createShare`, `updateShare`, `deleteShare`, `getPodcasts`, `getNewestPodcasts`, `refreshPodcasts`, `createPodcastChannel`, `deletePodcastChannel`, `deletePodcastEpisode`, `downloadPodcastEpisode`, `jukeboxControl`, `getInternetRadioStations`, `createInternetRadioStation`, `updateInternetRadioStation`, `deleteInternetRadioStation`, `getChatMessages`, `addChatMessage`, `createUser`, `updateUser`, `deleteUser`, `changePassword`, `getBookmarks`, `createBookmark`, `deleteBookmark`, `getPlayQueue`, `savePlayQueue`, `getScanStatus`, `startScan`

## Tag Editing

Write-through model: edit updates both the `songs` DB row and rewrites the file's ID3/Vorbis tags using the `id3` Rust crate (or `metaflac` for FLAC, `mp4ameta` for AAC).

Editable fields:
- Title, Artist, Album, Album Artist, Genre, Year, Track Number, Disc Number

Cover art editing: replace embedded cover art in the file + regenerate extracted cover in `.covers/`.

## Upload Flow

1. User selects audio files in Blackfiles UI (global library view).
2. Files are uploaded via TUS (reuse existing `tus.rs` infra, targeting `storage/music/`).
3. After upload completes, a post-processing job:
   - Reads ID3/Vorbis tags with `id3` crate.
   - Inserts row into `songs` table.
   - Extracts embedded cover art → `storage/music/.covers/<song_id>.{ext}`.
   - Reports any unreadable/corrupt files.
4. Song is immediately visible in the global library.

## Frontend Route Restructuring

The current settings pages move under `/admin/`. New personal settings pages are added under `/settings/`.

```
/                     ← existing (dashboard/redirect)
/browse               ← existing (file browser, scoped to storage/files/)
/music                ← new (music library — global + personal)
/login                ← existing
/upload/:token        ← existing
/upload-links         ← existing

/settings             ← new (personal settings hub)
/settings/general     ← new (profile info, password change)
/settings/api-keys    ← new (manage own OpenSubsonic API keys)

/admin                ← new (admin hub, was /settings)
/admin/users          ← migrated from /settings/users
/admin/roles          ← migrated from /settings/roles
/admin/api-keys       ← new (admin overview of all users' API keys)
```

### Migration

- Move `/settings/users` → `/admin/users`
- Move `/settings/roles` → `/admin/roles`
- Add redirects from old paths (or update sidebar links atomically)

## Implementation Phases

### Phase 0 — Foundation

- [x] Move `STORAGE_ROOT` from `storage/` to `storage/files/` (migrate existing files).
- [x] Add `MUSIC_ROOT = "storage/music/"` constant.
- [x] Create DB migration `0006_music_library.sql` with all schema above.
- [x] Seed new permissions (`music_upload`, `music_delete`, `music_edit_tags`, `music_manage_api_keys`, `music_manage_all_api_keys`).
- [x] Add `id3` crate to `Cargo.toml`.

### Phase 1 — Upload & Tag Scanning

- [x] TUS upload handler for `storage/music/` (`/api/music/uploads`).
- [x] Post-upload tag scanner: `lofty` → `songs` row + cover art extraction.
- [x] Blackfiles music API: `GET /api/music/songs`, `DELETE /api/music/songs/<id>`, `PUT /api/music/songs/<id>/tags`.
- [x] `POST /api/music/scan` for re-scanning existing files.
- [x] Personal library: `GET /api/music/library`, `POST/DELETE /api/music/library/<song_id>`.

### Phase 2 — API Keys

- [x] `GET/POST/DELETE /api/music/api-keys` endpoints (user manages own keys).
- [x] `GET/DELETE /api/admin/api-keys` endpoints (admin manages all keys).
- [x] API key auth guard (`ApiKeyUser`) for `/rest/` OpenSubsonic endpoints.

### Phase 3 — OpenSubsonic Scaffolding

- [x] `/rest/` router (separate from `/api/`).
- [x] `subsonic-response` envelope (JSON via `f` param).
- [x] Auth middleware: `SubsonicUser` guard supports `apiKey`, `u`+`p`; `t`+`s` rejected (argon2 incompatibility).
- [x] `ping`, `getLicense`, `getOpenSubsonicExtensions` endpoints.

### Phase 4 — OpenSubsonic Browsing

- [x] `getMusicFolders`, `getIndexes`, `getMusicDirectory`.
- [x] `getArtists`, `getArtist`, `getAlbum`, `getSong`.
- [x] `getAlbumList`/`getAlbumList2`, `getGenres`.

### Phase 5 — OpenSubsonic Media & Search

- [x] `stream` (with HTTP Range support).
- [x] `download`.
- [x] `getCoverArt`.
- [x] `search2`/`search3`.
- [x] `getRandomSongs`.

### Phase 6 — OpenSubsonic Playlists & Annotations

- [ ] `getPlaylists`, `getPlaylist`, `createPlaylist`, `updatePlaylist`, `deletePlaylist`.
- [ ] `star`/`unstar`, `getStarred`/`getStarred2`.
- [ ] `scrobble`, `getNowPlaying`.

### Phase 7 — Blackfiles Music UI & Settings Restructure

- [ ] Restructure frontend routes: `/settings/*` → personal, `/admin/*` → admin.
- [ ] Migrate `/settings/users` → `/admin/users`, `/settings/roles` → `/admin/roles`.
- [ ] New `/settings/general` page (profile info, password change).
- [ ] New `/settings/api-keys` page (list, create, revoke own API keys).
- [ ] New `/admin/api-keys` page (admin: list all keys by user, revoke any).
- [ ] New sidebar entry: "Music" (`/music` route).
- [ ] Global library view: table of songs (title, artist, album, genre, duration), search/filter.
- [ ] Upload button → file picker → TUS upload.
- [ ] Personal library view: same table, with "Add from Global" button/modal.
- [ ] Tag editor modal: inline edit of title/artist/album/genre/year/track.
- [ ] Delete confirmation with cascade warning.

### Phase 8 — Polish & Tier 2/3 Endpoints

- [ ] Remaining OpenSubsonic browsing/info endpoints.
- [ ] Stub responses for unsupported features (podcasts, jukebox, etc.).
- [ ] Transcoding support (optional; requires ffmpeg).
- [ ] Album art fallback (generated placeholder for albums without embedded art).

## Open Questions

1. **Flat vs hierarchical file layout?** `storage/music/{artist}/{album}/{track}.ext` is human-browsable. Flat UUID eliminates path collisions from weird artist names. The DB handles lookup either way. Recommendation: hierarchical — easier to debug.

2. **Transcoding?** OpenSubsonic `stream` accepts `format=mp3&maxBitRate=128`. Needs ffmpeg. Skip for MVP, add later.

3. **Duplicate detection?** On upload, check `file_path` uniqueness (DB constraint) and optionally fingerprint (acoustic ID) for true duplicates. MVP: just file path uniqueness.

4. **`music/` in Blackfiles storage UI?** The existing file browser at `/browse` should NOT show `storage/music/`. It should be scoped to `storage/files/` only. Music has its own UI.
