-- Migration: create user statistics table
--
-- Pre-aggregated user statistics for efficient query performance.
-- Maintains incremental aggregates via triggers to avoid full-table scans
-- for common queries (user betting volume, winnings, prediction counts).
--
-- Tables:
-- - user_stats: Main user statistics table with betting volume and prediction counts
-- - user_pool_stats: Per-pool statistics for each user
-- - user_outcomes: Tracks outcomes on which user has placed predictions

CREATE TABLE IF NOT EXISTS user_stats (
    user_address    VARCHAR(56)     PRIMARY KEY,
    total_volume    NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    total_winnings  NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    prediction_count BIGINT         NOT NULL DEFAULT 0,
    active_pools    BIGINT          NOT NULL DEFAULT 0,
    last_prediction TIMESTAMPTZ,
    last_updated    TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_pool_stats (
    user_address    VARCHAR(56)     NOT NULL,
    pool_id         BIGINT          NOT NULL,
    stake_amount    NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome         INTEGER,
    prediction_count BIGINT         NOT NULL DEFAULT 0,
    last_updated    TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_address, pool_id),
    FOREIGN KEY (pool_id) REFERENCES pools (pool_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_outcomes (
    user_address    VARCHAR(56)     NOT NULL,
    pool_id         BIGINT          NOT NULL,
    outcome         INTEGER         NOT NULL,
    stake_count     BIGINT          NOT NULL DEFAULT 1,
    last_updated    TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_address, pool_id, outcome),
    FOREIGN KEY (pool_id) REFERENCES pools (pool_id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_user_stats_total_volume ON user_stats (total_volume DESC);
CREATE INDEX IF NOT EXISTS idx_user_stats_total_winnings ON user_stats (total_winnings DESC);
CREATE INDEX IF NOT EXISTS idx_user_stats_prediction_count ON user_stats (prediction_count DESC);
CREATE INDEX IF NOT EXISTS idx_user_stats_last_prediction ON user_stats (last_prediction DESC);
CREATE INDEX IF NOT EXISTS idx_user_pool_stats_pool_id ON user_pool_stats (pool_id);
CREATE INDEX IF NOT EXISTS idx_user_pool_stats_stake ON user_pool_stats (stake_amount DESC);
CREATE INDEX IF NOT EXISTS idx_user_outcomes_pool_id ON user_outcomes (pool_id);

-- Backfill user_stats from existing predictions
INSERT INTO user_stats (
    user_address,
    total_volume,
    prediction_count,
    last_prediction,
    last_updated
)
SELECT
    user_address,
    COALESCE(SUM(amount), 0)::NUMERIC(32, 7),
    COUNT(*)::BIGINT,
    MAX(created_at),
    NOW()
FROM predictions
GROUP BY user_address
ON CONFLICT (user_address) DO UPDATE SET
    total_volume = EXCLUDED.total_volume,
    prediction_count = EXCLUDED.prediction_count,
    last_prediction = EXCLUDED.last_prediction,
    last_updated = EXCLUDED.last_updated;

-- Backfill user_pool_stats from existing predictions
INSERT INTO user_pool_stats (
    user_address,
    pool_id,
    stake_amount,
    outcome,
    prediction_count,
    last_updated
)
SELECT
    user_address,
    pool_id,
    COALESCE(SUM(amount), 0)::NUMERIC(32, 7),
    outcome,
    COUNT(*)::BIGINT,
    NOW()
FROM predictions
GROUP BY user_address, pool_id, outcome
ON CONFLICT (user_address, pool_id) DO UPDATE SET
    stake_amount = EXCLUDED.stake_amount,
    outcome = EXCLUDED.outcome,
    prediction_count = EXCLUDED.prediction_count,
    last_updated = EXCLUDED.last_updated;

-- Backfill user_outcomes from existing predictions
INSERT INTO user_outcomes (
    user_address,
    pool_id,
    outcome,
    stake_count,
    last_updated
)
SELECT
    user_address,
    pool_id,
    outcome,
    COUNT(*)::BIGINT,
    NOW()
FROM predictions
GROUP BY user_address, pool_id, outcome
ON CONFLICT (user_address, pool_id, outcome) DO UPDATE SET
    stake_count = EXCLUDED.stake_count,
    last_updated = EXCLUDED.last_updated;

-- Trigger function to maintain user_stats when predictions are inserted
CREATE OR REPLACE FUNCTION maintain_user_stats()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert or update user stats
    INSERT INTO user_stats (
        user_address,
        total_volume,
        prediction_count,
        last_prediction,
        last_updated
    )
    VALUES (
        NEW.user_address,
        NEW.amount::NUMERIC(32, 7),
        1,
        NEW.created_at,
        NOW()
    )
    ON CONFLICT (user_address) DO UPDATE SET
        total_volume = user_stats.total_volume + NEW.amount,
        prediction_count = user_stats.prediction_count + 1,
        last_prediction = NEW.created_at,
        last_updated = NOW();

    -- Insert or update user pool stats
    INSERT INTO user_pool_stats (
        user_address,
        pool_id,
        stake_amount,
        outcome,
        prediction_count,
        last_updated
    )
    VALUES (
        NEW.user_address,
        NEW.pool_id,
        NEW.amount::NUMERIC(32, 7),
        NEW.outcome,
        1,
        NOW()
    )
    ON CONFLICT (user_address, pool_id) DO UPDATE SET
        stake_amount = user_pool_stats.stake_amount + NEW.amount,
        prediction_count = user_pool_stats.prediction_count + 1,
        last_updated = NOW();

    -- Insert or update user outcomes
    INSERT INTO user_outcomes (
        user_address,
        pool_id,
        outcome,
        stake_count,
        last_updated
    )
    VALUES (
        NEW.user_address,
        NEW.pool_id,
        NEW.outcome,
        1,
        NOW()
    )
    ON CONFLICT (user_address, pool_id, outcome) DO UPDATE SET
        stake_count = user_outcomes.stake_count + 1,
        last_updated = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger on predictions insert to maintain user statistics
DROP TRIGGER IF EXISTS trg_predictions_maintain_user_stats ON predictions;
CREATE TRIGGER trg_predictions_maintain_user_stats
    AFTER INSERT ON predictions
    FOR EACH ROW
    EXECUTE FUNCTION maintain_user_stats();
