-- Migration: add index to created_at for pools
--
-- Optimizes sorting by creation time.

CREATE INDEX IF NOT EXISTS idx_pools_created_at ON pools (created_at);
