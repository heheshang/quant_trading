-- 订单来源/种类：纸面(paper) / 实盘(live) / 算法(algorithm) 等。
-- 活跃/历史订单可据此区分纸面与实盘（含其他来源）。
ALTER TABLE orders ADD COLUMN IF NOT EXISTS exchange VARCHAR(20) NOT NULL DEFAULT 'paper';
CREATE INDEX IF NOT EXISTS idx_orders_exchange ON orders(exchange);
