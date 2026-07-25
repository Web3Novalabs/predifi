-- Migration: optimize indexes for user_address columns
--
-- Speeds up user-scoped prediction history queries (filter + ORDER BY created_at)
-- and referral lookups keyed by referred user address.

CREATE INDEX IF NOT EXISTS idx_predictions_user_created
    ON predictions (user_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_referrals_user_address
    ON referrals (user_address);
