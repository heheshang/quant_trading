-- 逐资产余额（remote REST `get_balance` 由快照写入器 60s 落库；每资产一行，按 asset upsert）。
CREATE TABLE IF NOT EXISTS balances (
    asset TEXT PRIMARY KEY,
    free NUMERIC(20, 8) NOT NULL DEFAULT 0,
    locked NUMERIC(20, 8) NOT NULL DEFAULT 0,
    ts TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 全标的最近价（remote REST `get_all_ticker_prices` 由快照写入器 60s 落库；domain symbol 为 PK）。
-- 供纸面定价/余额市值/任一标的取价，不再依赖前端直连币安 REST。
CREATE TABLE IF NOT EXISTS last_prices (
    symbol TEXT PRIMARY KEY,
    price NUMERIC(20, 8) NOT NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
