-- 逐笔成交流（remote WS `@trade` 导入）；按 symbol+trade_time 查询最近成交。
CREATE TABLE IF NOT EXISTS stream_trades (
    id BIGSERIAL PRIMARY KEY,
    symbol TEXT NOT NULL,
    price NUMERIC(20, 8) NOT NULL,
    quantity NUMERIC(20, 8) NOT NULL,
    trade_time TIMESTAMPTZ NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_stream_trades_symbol_time ON stream_trades(symbol, trade_time DESC);

-- 订单簿最新快照（remote WS `@depth`/`@orderbook` 导入）；每标的一行，按 symbol upsert。
-- 高频 depth 不落历史明细，只保留最近一帧，避免行数无界膨胀。
CREATE TABLE IF NOT EXISTS orderbook_snapshots (
    symbol TEXT PRIMARY KEY,
    bids TEXT NOT NULL DEFAULT '[]',
    asks TEXT NOT NULL DEFAULT '[]',
    ts TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
