-- Migration 015: Add optimistic lock version column to strategies table.
-- Slot 012 is reserved and must not be reused (see .harness/wiki/migration-naming.md).
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 0;
