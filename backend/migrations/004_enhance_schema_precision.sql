-- Migration: Enhance schema with precise numeric types and add stats table
--
-- This migration addresses issue #706: PostgreSQL Schema Definition & Migrations
-- 
-- Changes:
-- 1. Alter amount columns to use NUMERIC(32, 7) for precision
-- 2. Add contract_id column to pools table
-- 3. Create stats table for aggregated pool statistics
-- 4. Add indexes for performance optimization

-- ── Alter existing tables for precision ──────────────────────────────────────

-- Update pools table: change total_stake to NUMERIC for precision
ALTER TABLE pools 
    ALTER COLUMN total_stake TYPE NUMERIC(32, 7) USING total_stake::NUMERIC(32, 7);

-- Add contract_id column to pools table for tracking on-chain contract address
ALTER TABLE pools 
    ADD COLUMN IF NOT EXISTS contract_id TEXT;

-- Update predictions table: change amount to NUMERIC for precision
ALTER TABLE predictions 
    ALTER COLUMN amount TYPE NUMERIC(32, 7) USING amount::NUMERIC(32, 7);

-- Update referrals table: change amount to NUMERIC for precision
ALTER TABLE referrals 
    ALTER COLUMN amount TYPE NUMERIC(32, 7) USING amount::NUMERIC(32, 7);

-- ── Create stats table ───────────────────────────────────────────────────────

-- Aggregated statistics per pool for efficient querying
CREATE TABLE IF NOT EXISTS stats (
    pool_id         BIGINT          PRIMARY KEY REFERENCES pools (pool_id) ON DELETE CASCADE,
    total_stake     NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    total_predictions BIGINT        NOT NULL DEFAULT 0,
    unique_users    BIGINT          NOT NULL DEFAULT 0,
    outcome_0_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_1_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_2_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_3_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_4_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_5_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_6_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    outcome_7_stake NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    last_updated    TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- Index for efficient stats queries
CREATE INDEX IF NOT EXISTS idx_stats_last_updated ON stats (last_updated);

-- ── Add performance indexes ──────────────────────────────────────────────────

-- Composite index for filtering pools by state and category
CREATE INDEX IF NOT EXISTS idx_pools_state_category ON pools (state, category);

-- Index for sorting pools by end_time
CREATE INDEX IF NOT EXISTS idx_pools_end_time ON pools (end_time);

-- Index for sorting pools by total_stake (popular pools)
CREATE INDEX IF NOT EXISTS idx_pools_total_stake ON pools (total_stake DESC);

-- Composite index for predictions queries
CREATE INDEX IF NOT EXISTS idx_predictions_pool_outcome ON predictions (pool_id, outcome);

-- ── Create function to update stats ──────────────────────────────────────────

-- Function to recalculate stats for a specific pool
CREATE OR REPLACE FUNCTION update_pool_stats(p_pool_id BIGINT)
RETURNS VOID AS $$
BEGIN
    INSERT INTO stats (
        pool_id,
        total_stake,
        total_predictions,
        unique_users,
        outcome_0_stake,
        outcome_1_stake,
        outcome_2_stake,
        outcome_3_stake,
        outcome_4_stake,
        outcome_5_stake,
        outcome_6_stake,
        outcome_7_stake,
        last_updated
    )
    SELECT
        p_pool_id,
        COALESCE(SUM(amount), 0) as total_stake,
        COUNT(*) as total_predictions,
        COUNT(DISTINCT user_address) as unique_users,
        COALESCE(SUM(CASE WHEN outcome = 0 THEN amount ELSE 0 END), 0) as outcome_0_stake,
        COALESCE(SUM(CASE WHEN outcome = 1 THEN amount ELSE 0 END), 0) as outcome_1_stake,
        COALESCE(SUM(CASE WHEN outcome = 2 THEN amount ELSE 0 END), 0) as outcome_2_stake,
        COALESCE(SUM(CASE WHEN outcome = 3 THEN amount ELSE 0 END), 0) as outcome_3_stake,
        COALESCE(SUM(CASE WHEN outcome = 4 THEN amount ELSE 0 END), 0) as outcome_4_stake,
        COALESCE(SUM(CASE WHEN outcome = 5 THEN amount ELSE 0 END), 0) as outcome_5_stake,
        COALESCE(SUM(CASE WHEN outcome = 6 THEN amount ELSE 0 END), 0) as outcome_6_stake,
        COALESCE(SUM(CASE WHEN outcome = 7 THEN amount ELSE 0 END), 0) as outcome_7_stake,
        NOW()
    FROM predictions
    WHERE pool_id = p_pool_id
    ON CONFLICT (pool_id) DO UPDATE SET
        total_stake = EXCLUDED.total_stake,
        total_predictions = EXCLUDED.total_predictions,
        unique_users = EXCLUDED.unique_users,
        outcome_0_stake = EXCLUDED.outcome_0_stake,
        outcome_1_stake = EXCLUDED.outcome_1_stake,
        outcome_2_stake = EXCLUDED.outcome_2_stake,
        outcome_3_stake = EXCLUDED.outcome_3_stake,
        outcome_4_stake = EXCLUDED.outcome_4_stake,
        outcome_5_stake = EXCLUDED.outcome_5_stake,
        outcome_6_stake = EXCLUDED.outcome_6_stake,
        outcome_7_stake = EXCLUDED.outcome_7_stake,
        last_updated = NOW();
END;
$$ LANGUAGE plpgsql;

-- ── Populate stats for existing pools ────────────────────────────────────────

-- Initialize stats for all existing pools
INSERT INTO stats (pool_id, total_stake, total_predictions, unique_users, last_updated)
SELECT 
    pool_id,
    COALESCE(total_stake, 0),
    0,
    0,
    NOW()
FROM pools
ON CONFLICT (pool_id) DO NOTHING;
