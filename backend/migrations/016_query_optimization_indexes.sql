-- Migration: indexes found missing by the db query audit (#2204)
--
-- Every index below backs a specific query. None of them is created
-- CONCURRENTLY because migrations run inside a transaction (see
-- `migration_checks`); all are IF NOT EXISTS so the migration is re-runnable.

-- 1. Cursor pagination of a pool's predictions.
--    db::predictions::get_market_predictions:
--      WHERE pool_id = $1 AND id < $2 ORDER BY id DESC LIMIT n
--    The existing (pool_id, created_at DESC) index cannot satisfy ORDER BY id,
--    so Postgres had to read and sort every prediction of the pool. This index
--    returns rows already ordered and also makes COUNT(*) WHERE pool_id = $1
--    (count_market_predictions, get_pool_with_odds) an index-only scan.
CREATE INDEX IF NOT EXISTS idx_predictions_pool_id_desc
    ON predictions (pool_id, id DESC);

-- 2. Covering index for per-user aggregates.
--    Serves, without touching the heap:
--      * get_users_by_betting_volume  (GROUP BY user_address, SUM(amount))
--      * profile::get_profile_stats / get_performance_over_time /
--        get_user_claim_status         (WHERE user_address = $1)
--    idx_predictions_amount_desc (migration 009) cannot help a
--    GROUP BY user_address, and the plain user_address indexes force a heap
--    fetch per row for outcome / amount / claimed_amount.
CREATE INDEX IF NOT EXISTS idx_predictions_user_covering
    ON predictions (user_address)
    INCLUDE (pool_id, outcome, amount, claimed, claimed_amount, created_at);

-- 3. Notification sweep: "recently resolved / claim-expiring pools".
--    notifications::run_notification_sweep filters settled pools by
--    resolved_at every few minutes; without this it scans all settled pools.
CREATE INDEX IF NOT EXISTS idx_pools_settled_resolved_at
    ON pools (resolved_at)
    WHERE state = 'settled' AND result IS NOT NULL;

-- 4. Notification sweep: interest matching.
--    WHERE interest = ANY($1) — the primary key leads with user_address, so it
--    cannot be used to look up by interest.
CREATE INDEX IF NOT EXISTS idx_user_interests_interest
    ON user_interests (interest, user_address);
