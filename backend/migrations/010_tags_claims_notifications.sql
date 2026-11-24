-- Migration: pool tags, claim tracking, and notification system
--
-- Adds:
-- 1. Multi-tag support on pools (category remains the single primary
--    classification; tags is an array for finer-grained filtering).
-- 2. Claim tracking on predictions so profile pages can show claim status,
--    plus resolution metadata on pools so a per-pool claim window can be
--    computed without round-tripping to the contract.
-- 3. A notifications table + per-user interest list so the backend can alert
--    users about pools ending soon, resolutions, expiring claim windows, and
--    new pools matching their interests.

-- ── Tags ───────────────────────────────────────────────────────────────────

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS tags TEXT[] NOT NULL DEFAULT '{}';

-- GIN index for `tags && $1` overlap filtering.
CREATE INDEX IF NOT EXISTS idx_pools_tags ON pools USING GIN (tags);

-- ── Claim tracking ───────────────────────────────────────────────────────────

ALTER TABLE predictions
    ADD COLUMN IF NOT EXISTS claimed        BOOLEAN        NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS claimed_amount NUMERIC(32, 7) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS claimed_at     TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_predictions_unclaimed
    ON predictions (pool_id, user_address)
    WHERE NOT claimed;

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS resolved_at          TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS claim_window_seconds  BIGINT NOT NULL DEFAULT 2592000; -- 30 days, matches contract default

-- ── Notifications ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS notifications (
    id           BIGSERIAL    PRIMARY KEY,
    user_address TEXT         NOT NULL,
    notif_type   VARCHAR(32)  NOT NULL
                              CHECK (notif_type IN (
                                  'pool_ending_soon',
                                  'pool_resolved',
                                  'claim_expiring',
                                  'new_pool_match'
                              )),
    title        TEXT         NOT NULL,
    message      TEXT         NOT NULL,
    pool_id      BIGINT       REFERENCES pools (pool_id) ON DELETE CASCADE,
    read         BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_created
    ON notifications (user_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_notifications_user_unread
    ON notifications (user_address)
    WHERE NOT read;

-- One notification per (user, pool, type): sweeps and event ingestion can run
-- repeatedly and rely on `ON CONFLICT DO NOTHING` for idempotency.
CREATE UNIQUE INDEX IF NOT EXISTS idx_notifications_dedupe
    ON notifications (user_address, pool_id, notif_type)
    WHERE pool_id IS NOT NULL;

-- ── User interests (for "new pools matching interests" alerts) ───────────────

CREATE TABLE IF NOT EXISTS user_interests (
    user_address TEXT NOT NULL,
    interest     TEXT NOT NULL, -- matched against pools.category and pools.tags
    PRIMARY KEY (user_address, interest)
);
