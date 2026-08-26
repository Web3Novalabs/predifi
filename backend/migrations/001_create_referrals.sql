-- Migration: create referrals table
--
-- Tracks on-chain referral events indexed off-chain by the backend.
-- Each row represents one referral: a user who staked in a pool via a referrer link.

CREATE TABLE IF NOT EXISTS referrals (
    id            BIGSERIAL    PRIMARY KEY,
    referrer      TEXT         NOT NULL,          -- referrer wallet address
    user_address  TEXT         NOT NULL,          -- referred user wallet address
    pool_id       BIGINT       NOT NULL,          -- prediction pool ID
    amount        BIGINT       NOT NULL DEFAULT 0, -- stake amount in stroops/base units
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_referrals_referrer ON referrals (referrer);
