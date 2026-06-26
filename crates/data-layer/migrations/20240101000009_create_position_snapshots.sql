CREATE TABLE IF NOT EXISTS position_snapshots (
    id BIGSERIAL,
    inst_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    pos NUMERIC,
    avg_px NUMERIC,
    upl NUMERIC,
    upl_ratio NUMERIC,
    mark_px NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (inst_id, ts)
);
