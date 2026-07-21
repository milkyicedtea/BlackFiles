-- Authenticated resumable uploads feature.
BEGIN;

CREATE TABLE IF NOT EXISTS upload_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_path TEXT NOT NULL UNIQUE,
    upload_length BIGINT NOT NULL CHECK (upload_length >= 0),
    upload_offset BIGINT NOT NULL DEFAULT 0 CHECK (upload_offset >= 0 AND upload_offset <= upload_length),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_upload_sessions_expires_at ON upload_sessions(expires_at);

COMMIT;
