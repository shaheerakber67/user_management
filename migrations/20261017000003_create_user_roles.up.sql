-- Many-to-many mapping between users and roles.
CREATE TABLE user_roles (
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id     SMALLINT    NOT NULL REFERENCES roles (id) ON DELETE RESTRICT,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

-- The PK covers lookups by user_id; this covers "all users with role X".
CREATE INDEX user_roles_role_id_idx ON user_roles (role_id);
