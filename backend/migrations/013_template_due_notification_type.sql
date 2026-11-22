-- Migration: allow 'template_due' as a notifications.notif_type (#1935)
--
-- The pool-template auto-creation scheduler (backend/src/worker/template_scheduler.rs)
-- notifies a creator when their recurring pool template is due to be
-- re-instantiated. `notifications.notif_type` has a CHECK constraint
-- allow-listing the four values introduced in migration 010; this adds the
-- fifth. Postgres names an inline, unnamed CHECK after
-- `<table>_<column>_check` by default, which is the constraint migration
-- 010 created — drop and recreate it with the extra value.

ALTER TABLE notifications DROP CONSTRAINT IF EXISTS notifications_notif_type_check;

ALTER TABLE notifications ADD CONSTRAINT notifications_notif_type_check
    CHECK (notif_type IN (
        'pool_ending_soon',
        'pool_resolved',
        'claim_expiring',
        'new_pool_match',
        'template_due'
    ));
