BEGIN;

LOCK TABLE roles IN ACCESS EXCLUSIVE MODE;

ALTER TABLE roles RENAME COLUMN "order" TO position;
ALTER TABLE roles RENAME CONSTRAINT roles_hierarchy_not_null TO roles_position_not_null;
ALTER TABLE roles ALTER COLUMN position DROP DEFAULT;

WITH ranked AS (
    SELECT
        r.id,
        ROW_NUMBER() OVER (
            ORDER BY
                r.position DESC,
                (
                    SELECT COUNT(*)
                    FROM role_permissions AS rp
                    WHERE rp.role_id = r.id
                ) DESC,
                r.name ASC
        )::integer AS normalized_position
    FROM roles AS r
)
UPDATE roles AS r
SET position = ranked.normalized_position
FROM ranked
WHERE r.id = ranked.id;

ALTER TABLE roles
    ADD CONSTRAINT roles_position_positive CHECK (position > 0),
    ADD CONSTRAINT roles_position_key UNIQUE (position) DEFERRABLE INITIALLY IMMEDIATE;

COMMIT;
