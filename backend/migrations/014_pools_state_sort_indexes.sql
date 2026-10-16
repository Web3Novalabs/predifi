-- Migration: composite (state, sort-column) indexes for pool listing (#1937)
--
-- `db::pools::get_pools_with_filters` / `count_pools_with_filters` back
-- `GET /api/v1/pools` — almost certainly the single most frequently hit
-- read in the whole API — and always filter on `state` (defaulting to
-- 'active') before applying one of three ORDER BY clauses chosen by
-- `sort_by`:
--
--   sort_by = "popular"      -> ORDER BY total_stake DESC
--   sort_by = "ending_soon"  -> ORDER BY end_time ASC
--   sort_by = "new"/default  -> ORDER BY created_at DESC
--
-- `pools` already has single-column indexes on each sort column
-- (idx_pools_total_stake, idx_pools_end_time, idx_pools_created_at) and on
-- state/category separately (idx_pools_state, idx_pools_state_category),
-- but none of them lead with `state` *and* match the sort column, so the
-- planner either does a bitmap AND of two indexes or scans one index and
-- re-sorts. The common no-category-filter case (the overwhelming majority
-- of real traffic — category is an optional drill-down) can instead use a
-- single composite index that satisfies the filter and returns rows
-- already in the required order, with no extra sort step.

CREATE INDEX IF NOT EXISTS idx_pools_state_created_at
    ON pools (state, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_pools_state_total_stake
    ON pools (state, total_stake DESC);

CREATE INDEX IF NOT EXISTS idx_pools_state_end_time
    ON pools (state, end_time ASC);
