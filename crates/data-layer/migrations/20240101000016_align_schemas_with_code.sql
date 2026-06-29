-- Migration 016: Align schemas with code queries (risk_config, market_data)
--
-- Background:
--   (1) risk_service.rs queries/updates enable_pre_trade_check and
--       enable_real_time_monitor columns that did not exist on the risk_config
--       table. Adding them with safe defaults preserves existing rows.
--   (2) The domain `MarketData` struct (domain/src/types.rs) carries turnover,
--       open_interest, and 5-level bid/ask arrays. The market_data table only
--       stored basic OHLCV. Adding nullable columns lets writes populate them
--       without breaking readers that don't bind them.
--   (3) market_data_repo.rs uses `ON CONFLICT (instrument_id, timeframe,
--       timestamp) DO NOTHING` in its insert path, but the table has no UNIQUE
--       covering that triple. The constraint must include the partition key
--       `timestamp` (PostgreSQL rule for partitioned tables).
--
-- All additions are idempotent (IF NOT EXISTS / DO block) so this migration is
-- safe to re-run on partially-migrated databases.

-- ─── 1. risk_config: enable_* feature flags ──────────────────────────────
ALTER TABLE risk_config
  ADD COLUMN IF NOT EXISTS enable_pre_trade_check BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS enable_real_time_monitor BOOLEAN NOT NULL DEFAULT TRUE;

-- Backfill: make sure the singleton id=1 row has the flags set (idempotent).
UPDATE risk_config
   SET enable_pre_trade_check = COALESCE(enable_pre_trade_check, TRUE),
       enable_real_time_monitor = COALESCE(enable_real_time_monitor, TRUE)
 WHERE id = 1;

-- ─── 2. market_data: extended K-line + order-book columns ────────────────
ALTER TABLE market_data
  ADD COLUMN IF NOT EXISTS turnover      DECIMAL(20, 8),
  ADD COLUMN IF NOT EXISTS open_interest DECIMAL(20, 8),
  ADD COLUMN IF NOT EXISTS bid_prices    DECIMAL(20, 8)[],
  ADD COLUMN IF NOT EXISTS bid_volumes   DECIMAL(20, 8)[],
  ADD COLUMN IF NOT EXISTS ask_prices    DECIMAL(20, 8)[],
  ADD COLUMN IF NOT EXISTS ask_volumes   DECIMAL(20, 8)[];

-- ─── 3. market_data: UNIQUE constraint to support ON CONFLICT upserts ────
-- On a RANGE-partitioned table, UNIQUE constraints MUST include the partition
-- key, which is `timestamp` here. That is why this is a 3-column constraint.
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
      FROM pg_constraint
     WHERE conname = 'market_data_instrument_timeframe_ts_key'
  ) THEN
    ALTER TABLE market_data
      ADD CONSTRAINT market_data_instrument_timeframe_ts_key
      UNIQUE (instrument_id, timeframe, timestamp);
  END IF;
END $$;
