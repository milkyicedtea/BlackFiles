-- Role ordering feature.
BEGIN;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'roles'
          AND column_name = 'order'
    ) THEN
        ALTER TABLE roles RENAME COLUMN "order" TO position;
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
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'roles_position_positive'
    ) THEN
        ALTER TABLE roles
            ADD CONSTRAINT roles_position_positive CHECK (position > 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'roles_position_key'
    ) THEN
        ALTER TABLE roles
            ADD CONSTRAINT roles_position_key UNIQUE (position) DEFERRABLE INITIALLY IMMEDIATE;
    END IF;
END;
$$;

COMMIT;
