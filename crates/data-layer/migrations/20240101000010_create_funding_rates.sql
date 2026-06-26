CREATE TABLE IF NOT EXISTS funding_rates (
    id BIGSERIAL,
    inst_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    funding_rate NUMERIC,
    next_funding_rate NUMERIC,
    funding_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (inst_id, ts)
);
