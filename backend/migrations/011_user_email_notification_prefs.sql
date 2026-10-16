-- Migration: user email addresses and notification delivery preferences
--
-- Adds:
-- 1. A `user_emails` table so users can opt in to email notifications.
-- 2. A `notification_delivery_log` table for idempotent email/push delivery
--    tracking: the sweep can run repeatedly and only send emails for
--    notifications it hasn't already delivered.

-- ── User email opt-in ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_emails (
    user_address    TEXT         NOT NULL PRIMARY KEY,
    email           TEXT         NOT NULL,
    -- Users must explicitly opt in per channel.
    email_enabled   BOOLEAN      NOT NULL DEFAULT TRUE,
    verified        BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_emails_email ON user_emails (email);

-- ── Delivery log ──────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS notification_delivery_log (
    notification_id BIGINT      NOT NULL REFERENCES notifications (id) ON DELETE CASCADE,
    channel         VARCHAR(16) NOT NULL CHECK (channel IN ('email', 'push')),
    sent_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (notification_id, channel)
);
