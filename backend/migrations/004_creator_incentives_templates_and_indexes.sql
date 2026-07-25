-- Migration: creator incentive system, pool templates, and query-optimization indexes
--
-- #1366 Pool creator incentive system:
--   Pool creators receive a share of the protocol fee once their pool's
--   total stake reaches a configured minimum participation threshold.
-- #1368 Pool templates for recurring markets:
--   Creators can save and reuse a pool configuration for recurring markets
--   (e.g. weekly BTC price predictions), with a next-run schedule.
-- #1369 / #1370 Query optimization:
--   Additional indexes for common pool/prediction lookup and sort patterns
--   that were previously unindexed.

-- ── #1366: creator incentive columns on pools ────────────────────────────────

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS creator_reward_bps          INTEGER     NOT NULL DEFAULT 500,
    ADD COLUMN IF NOT EXISTS min_participation_threshold BIGINT      NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS creator_reward_paid         BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS creator_reward_amount       BIGINT      NOT NULL DEFAULT 0;

-- Aggregate reputation / quality metrics per pool creator.
CREATE TABLE IF NOT EXISTS creator_stats (
    creator                TEXT         PRIMARY KEY,
    pools_created          BIGINT       NOT NULL DEFAULT 0,
    pools_reward_eligible  BIGINT       NOT NULL DEFAULT 0,
    total_volume           BIGINT       NOT NULL DEFAULT 0,
    updated_at             TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ── #1368: pool templates for recurring markets ──────────────────────────────

CREATE TABLE IF NOT EXISTS pool_templates (
    id                          BIGSERIAL    PRIMARY KEY,
    creator                     TEXT         NOT NULL,
    name                        TEXT         NOT NULL,
    category                    TEXT         NOT NULL DEFAULT '',
    description                 TEXT         NOT NULL DEFAULT '',
    token                       TEXT         NOT NULL DEFAULT '',
    duration_seconds            BIGINT       NOT NULL,
    recurrence_interval_seconds BIGINT       NOT NULL,
    next_run_at                 TIMESTAMPTZ  NOT NULL,
    active                      BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at                  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_templates_creator ON pool_templates (creator);

-- Partial index: the scheduler only ever scans active templates that are due.
CREATE INDEX IF NOT EXISTS idx_pool_templates_due
    ON pool_templates (next_run_at)
    WHERE active;

-- ── #1369 / #1370: additional indexes for hot query patterns ─────────────────

-- get_pools_with_filters: WHERE state = ... ORDER BY total_stake / end_time / created_at
CREATE INDEX IF NOT EXISTS idx_pools_total_stake ON pools (total_stake DESC);
CREATE INDEX IF NOT EXISTS idx_pools_end_time    ON pools (end_time ASC);
CREATE INDEX IF NOT EXISTS idx_pools_created_at  ON pools (created_at DESC);

-- Composite index covering the common (state, category) filter combination.
CREATE INDEX IF NOT EXISTS idx_pools_state_category ON pools (state, category);

-- Creator lookups (creator_stats joins / creator-owned pool listings).
CREATE INDEX IF NOT EXISTS idx_pools_creator ON pools (creator);

-- Partial index for get_users_by_winnings' settled/result-bearing pool scan.
CREATE INDEX IF NOT EXISTS idx_pools_settled_with_result
    ON pools (pool_id)
    WHERE state = 'settled' AND result IS NOT NULL;

-- get_user_prediction_history / get_user_predictions: WHERE user_address = ...
-- ORDER BY created_at DESC — composite index avoids a separate sort step.
CREATE INDEX IF NOT EXISTS idx_predictions_user_created
    ON predictions (user_address, created_at DESC);

-- get_pool_outcome_stakes: WHERE pool_id = ... GROUP BY outcome.
CREATE INDEX IF NOT EXISTS idx_predictions_pool_outcome
    ON predictions (pool_id, outcome);
