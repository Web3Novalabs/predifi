-- Migration: pool full-text search index and popularity counter
--
-- 1. `pool_search_text(name, category, tags)` — one immutable expression used
--    by both the GIN full-text index and the listing query, so the planner can
--    use the index. (`array_to_string` is only STABLE, hence the wrapper.)
-- 2. `pools.prediction_count` — denormalised number of predictions, maintained
--    on insert, so "sort by popularity" is an index-friendly column read
--    instead of a per-row COUNT over `predictions`.
-- 3. Supporting indexes for the new creator / token / case-insensitive
--    category filters.

CREATE OR REPLACE FUNCTION pool_search_text(p_name TEXT, p_category TEXT, p_tags TEXT[])
RETURNS TEXT
LANGUAGE SQL
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT coalesce(p_name, '') || ' ' || coalesce(p_category, '') || ' '
           || coalesce(array_to_string(p_tags, ' '), '')
$$;

CREATE INDEX IF NOT EXISTS idx_pools_search_fts
    ON pools USING GIN (to_tsvector('english', pool_search_text(name, category, tags)));

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS prediction_count BIGINT NOT NULL DEFAULT 0;

UPDATE pools p
SET prediction_count = c.cnt
FROM (
    SELECT pool_id, COUNT(*) AS cnt FROM predictions GROUP BY pool_id
) c
WHERE c.pool_id = p.pool_id;

CREATE INDEX IF NOT EXISTS idx_pools_state_prediction_count
    ON pools (state, prediction_count DESC);

CREATE INDEX IF NOT EXISTS idx_pools_creator ON pools (creator);
CREATE INDEX IF NOT EXISTS idx_pools_token ON pools (token);
CREATE INDEX IF NOT EXISTS idx_pools_lower_category ON pools (lower(category));
