-- Migration 017: Add 2026 monthly partitions to market_data
--
-- The original 20240101000003 migration only created partitions through
-- December 2025. The system clock is 2026-06-29, so writes for any month
-- in 2026 will fail with "no partition of relation 'market_data' found for
-- the row". This migration adds 2026-01 through 2026-12 inclusive.
--
-- All statements are idempotent (IF NOT EXISTS) so this file is safe to
-- re-run after partial application.

CREATE TABLE IF NOT EXISTS market_data_2026_01 PARTITION OF market_data
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS market_data_2026_02 PARTITION OF market_data
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS market_data_2026_03 PARTITION OF market_data
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS market_data_2026_04 PARTITION OF market_data
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS market_data_2026_05 PARTITION OF market_data
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS market_data_2026_06 PARTITION OF market_data
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE IF NOT EXISTS market_data_2026_07 PARTITION OF market_data
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE IF NOT EXISTS market_data_2026_08 PARTITION OF market_data
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS market_data_2026_09 PARTITION OF market_data
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE IF NOT EXISTS market_data_2026_10 PARTITION OF market_data
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE IF NOT EXISTS market_data_2026_11 PARTITION OF market_data
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE IF NOT EXISTS market_data_2026_12 PARTITION OF market_data
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
