CREATE TABLE inventory_servers (
    server_id UUID PRIMARY KEY,
    installation_id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    display_name TEXT NOT NULL,
    hostname TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    username TEXT NOT NULL,
    environment TEXT NOT NULL,
    approval_mode TEXT NOT NULL DEFAULT 'medium_and_above'
        CHECK (approval_mode IN ('medium_and_above', 'high_and_above')),
    revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX inventory_servers_owner_page_idx
    ON inventory_servers (installation_id, owner_user_id, updated_at, server_id);
