CREATE TABLE IF NOT EXISTS mark_prices (
    id BIGSERIAL,
    inst_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    mark_px NUMERIC,
    idx_px NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (inst_id, ts)
);
