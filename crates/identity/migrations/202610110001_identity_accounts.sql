CREATE TABLE identity_accounts (
    installation_id UUID NOT NULL,
    user_id UUID NOT NULL,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (installation_id, user_id),
    UNIQUE (installation_id, issuer, subject)
);
