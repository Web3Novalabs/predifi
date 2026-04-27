-- Migration: create pools table
--
-- Indexes prediction pool data from the Stellar/Soroban PrediFi contract.
-- Each row represents one on-chain prediction pool.

CREATE TABLE IF NOT EXISTS pools (
    id            BIGSERIAL     PRIMARY KEY,
    metadata_url  VARCHAR(2048) NOT NULL,
    start_time    TIMESTAMPTZ   NOT NULL,
    end_time      TIMESTAMPTZ   NOT NULL,
    status        VARCHAR(20)   NOT NULL DEFAULT 'Open'
                                CHECK (status IN ('Open', 'Closed', 'Settled')),
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),

    CONSTRAINT pools_end_after_start CHECK (end_time > start_time)
);

CREATE INDEX IF NOT EXISTS idx_pools_status ON pools (status);
