-- Allow NULL user_id in audit_logs for unauthenticated / system-originated
-- events (e.g. failed or anonymous logins). Previously `NOT NULL` + FK made a
-- placeholder `user_id=0` violate `audit_logs_user_id_fkey`, so those audit
-- rows were never persisted.

ALTER TABLE audit_logs ALTER COLUMN user_id DROP NOT NULL;
