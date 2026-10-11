CREATE TABLE inventory_credentials (
    credential_id UUID PRIMARY KEY,
    installation_id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    server_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('ssh_private_key', 'sudo_password')),
    envelope_version INTEGER NOT NULL,
    key_id TEXT NOT NULL,
    nonce BYTEA NOT NULL,
    ciphertext BYTEA NOT NULL,
    bound_at_revision BIGINT NOT NULL CHECK (bound_at_revision > 0),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    rotated_at TIMESTAMPTZ,
    CONSTRAINT inventory_credentials_server_fk
        FOREIGN KEY (server_id) REFERENCES inventory_servers (server_id)
);

CREATE UNIQUE INDEX inventory_credentials_active_kind_idx
    ON inventory_credentials (installation_id, owner_user_id, server_id, kind)
    WHERE enabled;
