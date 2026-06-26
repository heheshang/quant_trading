-- Sync backtest_results schema with code INSERT columns
-- The code INSERTs into columns that don't exist in the original schema.

ALTER TABLE backtest_results
    ADD COLUMN IF NOT EXISTS strategy_name VARCHAR(100),
    ADD COLUMN IF NOT EXISTS profit_loss_ratio DECIMAL(10, 4),
    ADD COLUMN IF NOT EXISTS winning_trades INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS losing_trades INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS equity_curve JSONB DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS symbols TEXT[] DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS commission_rate DECIMAL(10, 6) DEFAULT 0,
    ADD COLUMN IF NOT EXISTS slippage DECIMAL(10, 6) DEFAULT 0;

-- Down migration
-- ALTER TABLE backtest_results
--     DROP COLUMN IF EXISTS backtest_id,
--     DROP COLUMN IF EXISTS strategy_name,
--     DROP COLUMN IF EXISTS profit_loss_ratio,
--     DROP COLUMN IF EXISTS winning_trades,
--     DROP COLUMN IF EXISTS losing_trades,
--     DROP COLUMN IF EXISTS equity_curve,
--     DROP COLUMN IF EXISTS symbols,
--     DROP COLUMN IF EXISTS commission_rate,
--     DROP COLUMN IF EXISTS slippage;
