-- Migration: Add instance_label column to strategies table
-- Supports human-friendly naming for multiple strategy instances of the same type.
-- Uses IF NOT EXISTS so the migration is idempotent and safe to re-run on
-- databases where the column was already added manually (sqlx would otherwise
-- fail with "column already exists" and block all subsequent migrations).
ALTER TABLE strategies
    ADD COLUMN IF NOT EXISTS instance_label VARCHAR(200) DEFAULT NULL;
