-- Migration: optimize indexes for pool_id columns
--
-- Speeds up referral earnings grouped by pool and pool-scoped referral analytics.

CREATE INDEX IF NOT EXISTS idx_referrals_referrer_pool
    ON referrals (referrer, pool_id);

CREATE INDEX IF NOT EXISTS idx_referrals_pool_id
    ON referrals (pool_id);
