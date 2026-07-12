-- Seed default permissions, roles, and admin user

-- ══════════════════ PERMISSIONS ══════════════════

INSERT INTO permissions (name, display_name, group_name) VALUES
-- Files
('list_files',     'List files and directories',  'files'),
('download_files', 'Download files',              'files'),
('upload_files',   'Upload files',                'files'),
('delete_files',   'Delete files and directories', 'files'),

-- Users
('view_users',     'View users',   'users'),
('create_user',    'Create users', 'users'),
('edit_user',      'Edit users',   'users'),
('delete_user',    'Delete users', 'users'),

-- Roles
('view_roles',     'View roles',   'roles'),
('manage_roles',   'Manage roles', 'roles')
ON CONFLICT (name) DO NOTHING;

-- ══════════════════ ROLES ══════════════════

INSERT INTO roles (name, display_name, position, color) VALUES
('admin',  'Administrator', 1, 'red'),
('viewer', 'Viewer',         2, 'blue')
ON CONFLICT (name) DO NOTHING;

-- ══════════════════ ROLE PERMISSIONS ══════════════════

-- Admin gets all permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

-- Viewer gets read-only file access
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'viewer'
  AND p.name IN ('list_files', 'download_files')
ON CONFLICT DO NOTHING;

-- ══════════════════ DEFAULT ADMIN USER ══════════════════

-- The admin user is created at application startup if no users exist,
-- using the DEFAULT_ADMIN_PASSWORD env var. This is done in Rust code.
