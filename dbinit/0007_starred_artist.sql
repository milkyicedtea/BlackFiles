-- Allow starring artists (in addition to songs and albums).
-- The Phase 0 schema only permitted a song star OR an album star
-- (album_name + artist_name both required). This relaxes the CHECK so an
-- artist can be starred on its own (artist_name set, album_name NULL) and
-- adds a partial unique index so a user has at most one artist-only star per
-- artist (NULL album_name values are otherwise distinct in a plain UNIQUE).
BEGIN;

ALTER TABLE IF EXISTS starred DROP CONSTRAINT IF EXISTS starred_exactly_one_target;

ALTER TABLE starred ADD CONSTRAINT starred_exactly_one_target CHECK (
    (song_id IS NOT NULL AND album_name IS NULL AND artist_name IS NULL) OR
    (song_id IS NULL AND artist_name IS NOT NULL)
);

-- One artist-only star per (user, artist). The plain UNIQUE on
-- (user_id, artist_name, album_name) does not deduplicate NULL album_name
-- rows, so this partial index covers the artist-only case.
CREATE UNIQUE INDEX IF NOT EXISTS idx_starred_artist_only
    ON starred (user_id, artist_name)
    WHERE album_name IS NULL;

COMMIT;