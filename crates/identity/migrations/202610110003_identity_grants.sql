CREATE TABLE identity_grants (
    grant_id UUID PRIMARY KEY,
    installation_id UUID NOT NULL,
    user_id UUID NOT NULL,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    audience TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX identity_grants_owner_idx ON identity_grants (installation_id, user_id, expires_at);
