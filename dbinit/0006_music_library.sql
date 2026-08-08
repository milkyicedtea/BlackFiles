-- Music library schema.
BEGIN;

-- Songs (one row per audio file)
CREATE TABLE IF NOT EXISTS songs (
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

CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist);
CREATE INDEX IF NOT EXISTS idx_songs_album ON songs(album);
CREATE INDEX IF NOT EXISTS idx_songs_genre ON songs(genre);
CREATE INDEX IF NOT EXISTS idx_songs_title ON songs(title);

-- Personal library (junction)
CREATE TABLE IF NOT EXISTS user_songs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, song_id)
);


-- API keys for OpenSubsonic clients (raw key shown once, stored as SHA-256 hash)
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,
    label TEXT,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);

-- Playlists
CREATE TABLE IF NOT EXISTS playlists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    comment TEXT,
    public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS playlist_songs (
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, song_id)
);

-- Starred items
CREATE TABLE IF NOT EXISTS starred (
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
CREATE TABLE IF NOT EXISTS scrobbles (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    song_id UUID NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    played_at TIMESTAMPTZ NOT NULL,
    submission BOOLEAN DEFAULT TRUE
);
CREATE INDEX IF NOT EXISTS idx_scrobbles_user_id ON scrobbles(user_id);
CREATE INDEX IF NOT EXISTS idx_scrobbles_song_id ON scrobbles(song_id);

-- Trigger: auto-update songs.updated_at
DROP TRIGGER IF EXISTS update_songs_updated_at ON songs;
CREATE TRIGGER update_songs_updated_at
    BEFORE UPDATE ON songs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger: auto-update playlists.updated_at
DROP TRIGGER IF EXISTS update_playlists_updated_at ON playlists;
CREATE TRIGGER update_playlists_updated_at
    BEFORE UPDATE ON playlists
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMIT;
