-- Migration: create predictions table
--
-- Indexes individual user predictions (stakes) from the Stellar/Soroban PrediFi contract.
-- Each row represents one user's stake on an outcome within a pool.

CREATE TABLE IF NOT EXISTS predictions (
    id           BIGSERIAL    PRIMARY KEY,
    pool_id      BIGINT       NOT NULL REFERENCES pools (pool_id) ON DELETE RESTRICT,
    user_address VARCHAR(56)  NOT NULL,
    outcome      INTEGER      NOT NULL,
    amount       BIGINT       NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_predictions_pool_id      ON predictions (pool_id);
CREATE INDEX IF NOT EXISTS idx_predictions_user_address ON predictions (user_address);
