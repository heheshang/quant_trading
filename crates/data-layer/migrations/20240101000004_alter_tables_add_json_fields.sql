-- Migration 005: Add JSONB fields to backtest_results and strategies tables

ALTER TABLE backtest_results
ADD COLUMN IF NOT EXISTS parameters_json JSONB;

ALTER TABLE strategies
ADD COLUMN IF NOT EXISTS indicator_config_json JSONB;

-- Down migration
-- ALTER TABLE backtest_results DROP COLUMN IF EXISTS parameters_json;
-- ALTER TABLE strategies DROP COLUMN IF EXISTS indicator_config_json;
