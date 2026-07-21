-- Public resumable upload-links feature.
BEGIN;

ALTER TABLE upload_sessions
    ALTER COLUMN user_id DROP NOT NULL;

ALTER TABLE upload_sessions
    ADD COLUMN IF NOT EXISTS upload_link_id UUID REFERENCES upload_links(id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_upload_sessions_upload_link_id
    ON upload_sessions(upload_link_id)
    WHERE upload_link_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'upload_sessions_owner_check'
    ) THEN
        ALTER TABLE upload_sessions
            ADD CONSTRAINT upload_sessions_owner_check
            CHECK ((user_id IS NOT NULL) <> (upload_link_id IS NOT NULL));
    END IF;
END;
$$;

COMMIT;
