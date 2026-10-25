-- Roles available in the system. Seeded with the two built-in roles.
CREATE TABLE roles (
    id          SMALLSERIAL PRIMARY KEY,
    name        VARCHAR(32)  NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

INSERT INTO roles (name, description) VALUES
    ('user',    'Customer who browses menus and places food orders'),
    ('manager', 'Restaurant manager who manages menus and incoming orders');
