-- Add optimistic lock version column to strategies table
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 0;
