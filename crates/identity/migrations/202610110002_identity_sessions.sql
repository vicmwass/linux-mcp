CREATE TABLE identity_sessions (
    session_id UUID PRIMARY KEY,
    installation_id UUID NOT NULL,
    user_id UUID NOT NULL,
    csrf_hash BYTEA NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX identity_sessions_owner_idx ON identity_sessions (installation_id, user_id, expires_at);
