CREATE TABLE inventory_host_trust (
    host_trust_id UUID PRIMARY KEY,
    installation_id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    server_id UUID NOT NULL,
    hostname TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    status TEXT NOT NULL CHECK (status IN ('unenrolled', 'observed', 'verified', 'mismatch')),
    key_type TEXT,
    observed_fingerprint TEXT,
    verified_fingerprint TEXT,
    observed_at TIMESTAMPTZ,
    verified_at TIMESTAMPTZ,
    changed_at_revision BIGINT NOT NULL CHECK (changed_at_revision > 0),
    CONSTRAINT inventory_host_trust_server_fk
        FOREIGN KEY (server_id) REFERENCES inventory_servers (server_id)
);

CREATE UNIQUE INDEX inventory_host_trust_target_idx
    ON inventory_host_trust (installation_id, owner_user_id, server_id, hostname, port);
