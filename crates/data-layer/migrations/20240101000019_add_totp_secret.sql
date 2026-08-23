-- Two-factor authentication (TOTP) for users.
--
-- Idempotent append migration: adds the TOTP secret + enabled flag to the
-- existing `users` table. Existing migrations are never modified.
--
-- `totp_secret` holds the base32-encoded secret so the enable flow can first
-- provision a secret (challenge) and only mark the account protected after the
-- user has verified a live code (`verify_2fa_code`).
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;
