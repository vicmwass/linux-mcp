CREATE TABLE inventory_invalidation_events (
    event_id BIGSERIAL PRIMARY KEY,
    installation_id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    server_id UUID NOT NULL,
    revision BIGINT NOT NULL CHECK (revision > 0),
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX inventory_invalidations_owner_cursor_idx
    ON inventory_invalidation_events (installation_id, owner_user_id, event_id);
