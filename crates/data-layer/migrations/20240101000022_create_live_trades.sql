-- Live (Binance) order fills / metadata persistence.
--
-- Records each Binance order placed through the app (with its strategy link +
-- fill price/qty) so the UI can show strategy info and compute real P&L
-- without re-querying Binance per-asset (avoids rate-limit bans).
CREATE TABLE IF NOT EXISTS live_trades (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL UNIQUE,
    symbol TEXT NOT NULL,
    strategy_id TEXT NOT NULL DEFAULT '',
    side TEXT NOT NULL,
    price NUMERIC NOT NULL DEFAULT 0,
    quantity NUMERIC NOT NULL DEFAULT 0,
    filled_quantity NUMERIC NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_live_trades_symbol ON live_trades(symbol);
