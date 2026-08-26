-- Migration: create pools table
--
-- Indexes prediction pool data from the Stellar/Soroban PrediFi contract.
-- Each row represents one on-chain prediction pool.

CREATE TABLE IF NOT EXISTS pools (
    pool_id     BIGINT        PRIMARY KEY,
    name        TEXT          NOT NULL,
    category    TEXT          NOT NULL DEFAULT '',
    total_stake BIGINT        NOT NULL DEFAULT 0,
    end_time    TIMESTAMPTZ   NOT NULL,
    state       VARCHAR(20)   NOT NULL DEFAULT 'active'
                              CHECK (state IN ('active', 'closed', 'settled')),
    creator     TEXT          NOT NULL DEFAULT '',
    token       TEXT          NOT NULL DEFAULT '',
    result      TEXT,
    created_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pools_state    ON pools (state);
CREATE INDEX IF NOT EXISTS idx_pools_category ON pools (category);
