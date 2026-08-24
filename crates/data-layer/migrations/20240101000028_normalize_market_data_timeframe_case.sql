-- Migration 028: Canonicalize market_data.timeframe to lowercase.
--
-- The older data-puller wrote uppercase timeframes ("1H","1D") into
-- market_data; the WebSocket import, REST backfill, and frontend all read
-- lowercase ("1h","1d"). The legacy uppercase history was therefore invisible
-- to the K-line chart (get_klines('1h') never matched a '1H' row), which is
-- why the chart showed no data even though the table held a full year of bars.
--
-- This lowercases existing rows so all writers and readers agree on one case.
-- When the backfill created lowercase rows over the same bars as a legacy
-- uppercase row, the lowercase duplicate is removed (the legacy row is the
-- fuller historical bar) before lowercasing, to avoid a unique-violation on
-- (instrument_id, timeframe, timestamp).
--
-- Idempotent: rows already lowercase are untouched; re-running is a no-op.

-- Drop lowercase rows that duplicate a legacy uppercase row (same instrument,
-- same normalized timeframe, same timestamp).
DELETE FROM market_data AS l
USING market_data AS u
WHERE l.timeframe = lower(l.timeframe)
  AND u.timeframe <> lower(u.timeframe)
  AND l.instrument_id = u.instrument_id
  AND lower(l.timeframe) = lower(u.timeframe)
  AND l.timestamp = u.timestamp;

-- Lowercase the remaining legacy uppercase rows.
UPDATE market_data
SET timeframe = lower(timeframe)
WHERE timeframe <> lower(timeframe);
