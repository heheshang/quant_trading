-- Migration 025: Add 2027 monthly partitions to market_data.
--
-- Migration 017 only created partitions through 2026-12. Market data writes for
-- any month in 2027 will fail with "no partition of relation 'market_data' found
-- for the row". This migration adds 2027-01 through 2027-12 inclusive.
--
-- All statements are idempotent (IF NOT EXISTS) so this file is safe to re-run.

CREATE TABLE IF NOT EXISTS market_data_2027_01 PARTITION OF market_data
    FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE IF NOT EXISTS market_data_2027_02 PARTITION OF market_data
    FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE IF NOT EXISTS market_data_2027_03 PARTITION OF market_data
    FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');
CREATE TABLE IF NOT EXISTS market_data_2027_04 PARTITION OF market_data
    FOR VALUES FROM ('2027-04-01') TO ('2027-05-01');
CREATE TABLE IF NOT EXISTS market_data_2027_05 PARTITION OF market_data
    FOR VALUES FROM ('2027-05-01') TO ('2027-06-01');
CREATE TABLE IF NOT EXISTS market_data_2027_06 PARTITION OF market_data
    FOR VALUES FROM ('2027-06-01') TO ('2027-07-01');
CREATE TABLE IF NOT EXISTS market_data_2027_07 PARTITION OF market_data
    FOR VALUES FROM ('2027-07-01') TO ('2027-08-01');
CREATE TABLE IF NOT EXISTS market_data_2027_08 PARTITION OF market_data
    FOR VALUES FROM ('2027-08-01') TO ('2027-09-01');
CREATE TABLE IF NOT EXISTS market_data_2027_09 PARTITION OF market_data
    FOR VALUES FROM ('2027-09-01') TO ('2027-10-01');
CREATE TABLE IF NOT EXISTS market_data_2027_10 PARTITION OF market_data
    FOR VALUES FROM ('2027-10-01') TO ('2027-11-01');
CREATE TABLE IF NOT EXISTS market_data_2027_11 PARTITION OF market_data
    FOR VALUES FROM ('2027-11-01') TO ('2027-12-01');
CREATE TABLE IF NOT EXISTS market_data_2027_12 PARTITION OF market_data
    FOR VALUES FROM ('2027-12-01') TO ('2028-01-01');
