-- Migration: add targeted indexes on predictions table
--
-- Covers the query patterns that are not yet served by an index:
--
-- 1. idx_predictions_pool_created
--    Paginated prediction listings scoped to a single pool, ordered newest-first.
--    Supports: WHERE pool_id = $1 ORDER BY created_at DESC LIMIT n OFFSET m
--
-- 2. idx_predictions_outcome_pool
--    Winning-prediction lookup in the leaderboard CTE that filters on outcome
--    first then joins on pool_id.
--    Supports: WHERE p.outcome = CAST(pl.result AS INTEGER) [join on pool_id]
--
-- 3. idx_predictions_pool_user
--    Per-pool unique-user aggregation used by update_pool_stats() and the
--    leaderboard winnings calculation.
--    Supports: WHERE pool_id = $1 GROUP BY / COUNT(DISTINCT user_address)
--
-- 4. idx_predictions_amount_desc
--    Leaderboard query that ranks users by SUM(amount) DESC across the whole
--    table.  The partial DESC ordering allows the planner to avoid a sort step
--    when the GROUP BY aggregate is large.
--    Supports: SELECT user_address, SUM(amount) … ORDER BY SUM(amount) DESC

-- Index 1: pool-scoped pagination ordered by time
CREATE INDEX IF NOT EXISTS idx_predictions_pool_created
    ON predictions (pool_id, created_at DESC);

-- Index 2: outcome-first composite for winning-prediction joins
CREATE INDEX IF NOT EXISTS idx_predictions_outcome_pool
    ON predictions (outcome, pool_id);

-- Index 3: pool + user composite for unique-user aggregation
CREATE INDEX IF NOT EXISTS idx_predictions_pool_user
    ON predictions (pool_id, user_address);

-- Index 4: amount descending for betting-volume leaderboard scans
CREATE INDEX IF NOT EXISTS idx_predictions_amount_desc
    ON predictions (amount DESC);
