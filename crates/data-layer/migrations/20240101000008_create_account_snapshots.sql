CREATE TABLE IF NOT EXISTS account_snapshots (
    id BIGSERIAL,
    ccy TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    eq NUMERIC,
    cash_bal NUMERIC,
    avail_eq NUMERIC,
    frozen_bal NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (ccy, ts)
);
