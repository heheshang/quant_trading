-- Migration: Add instance_label column to strategies table
-- Supports human-friendly naming for multiple strategy instances of the same type.
ALTER TABLE strategies
    ADD COLUMN instance_label VARCHAR(200) DEFAULT NULL;
