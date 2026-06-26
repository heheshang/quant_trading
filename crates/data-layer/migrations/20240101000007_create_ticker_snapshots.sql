CREATE TABLE IF NOT EXISTS ticker_snapshots (
    id BIGSERIAL,
    instrument_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    last_px NUMERIC,
    open_24h NUMERIC,
    high_24h NUMERIC,
    low_24h NUMERIC,
    vol_24h NUMERIC,
    vol_ccy_24h NUMERIC,
    change_24h NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (instrument_id, ts)
);
