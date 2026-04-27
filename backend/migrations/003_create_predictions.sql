-- Migration: create predictions table
--
-- Indexes individual user predictions (stakes) from the Stellar/Soroban PrediFi contract.
-- Each row represents one user's stake on an outcome within a pool.
-- Foreign key to pools(id) prevents orphaned predictions.

CREATE TABLE IF NOT EXISTS predictions (
    id            BIGSERIAL    PRIMARY KEY,
    pool_id       BIGINT       NOT NULL REFERENCES pools (id) ON DELETE RESTRICT,
    user_address  VARCHAR(56)  NOT NULL,          -- Stellar ED25519 public key (G..., 56 chars)
    outcome       INTEGER      NOT NULL,           -- on-chain outcome index
    amount        BIGINT       NOT NULL DEFAULT 0, -- stake amount in stroops/base units
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_predictions_pool_id     ON predictions (pool_id);
CREATE INDEX IF NOT EXISTS idx_predictions_user_address ON predictions (user_address);
