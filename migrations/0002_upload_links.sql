BEGIN;

CREATE TABLE upload_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash CHAR(64) NOT NULL UNIQUE,
    target_path TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at TIMESTAMPTZ
);

CREATE INDEX idx_upload_links_created_by_user_id
    ON upload_links(created_by_user_id);

INSERT INTO permissions (name, display_name, group_name) VALUES
    ('create_upload_links', 'Create one-time upload links', 'upload_links'),
    ('view_upload_links',   'View one-time upload links',   'upload_links'),
    ('delete_upload_links', 'Delete one-time upload links', 'upload_links')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
JOIN permissions p ON p.name IN (
    'create_upload_links',
    'view_upload_links',
    'delete_upload_links'
)
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

COMMIT;
