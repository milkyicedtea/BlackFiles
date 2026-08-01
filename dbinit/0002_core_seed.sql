-- Core roles and permissions.
BEGIN;

INSERT INTO permissions (name, display_name, group_name) VALUES
    ('list_files',     'List files and directories',   'files'),
    ('download_files', 'Download files',               'files'),
    ('upload_files',   'Upload files',                 'files'),
    ('delete_files',   'Delete files and directories', 'files'),
    ('rename_files',   'Rename files and folders',     'files'),
    ('create_folders', 'Create folders',               'files'),
    ('view_users',     'View users',                   'users'),
    ('create_user',    'Create users',                 'users'),
    ('edit_user',      'Edit users',                   'users'),
    ('delete_user',    'Delete users',                 'users'),
    ('view_roles',     'View roles',                   'roles'),
    ('manage_roles',   'Manage roles',                 'roles'),
    ('music_upload',   'Upload music to global library','music'),
    ('music_delete',   'Delete music from global library','music'),
    ('music_edit_tags','Edit music tags (ID3/Vorbis)', 'music'),
    ('music_manage_api_keys',     'Manage own API keys',       'music'),
    ('music_manage_all_api_keys', 'Manage all users API keys', 'music')
ON CONFLICT (name) DO NOTHING;

INSERT INTO roles (name, display_name, position, color) VALUES
    ('admin',  'Administrator', 1, 'red'),
    ('viewer', 'Viewer',         2, 'blue')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'viewer'
  AND p.name IN ('list_files', 'download_files')
ON CONFLICT DO NOTHING;

COMMIT;
