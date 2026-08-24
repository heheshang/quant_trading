-- Persist per-trade records in backtest_results as JSONB.
--
-- The backtest engine now records each executed fill in `BacktestResult.trades`.
-- This column stores that list so reloading a result from history keeps the
-- trade records (previously they were only live in the run's response).

ALTER TABLE backtest_results
    ADD COLUMN IF NOT EXISTS trades JSONB NOT NULL DEFAULT '[]'::jsonb;
