-- Migration: capture per-outcome metadata for multi-outcome pools (#1932)
--
-- The on-chain `pool_created` event already carries `options_count` and
-- `outcome_descriptions` (see
-- contract/contracts/predifi-contract/src/events.rs::PoolCreatedEvent), but
-- the backend indexer previously discarded both fields when decoding the
-- event, so the API had nothing to serve beyond a bare numeric outcome
-- index. The frontend pool-detail view already renders an arbitrary-length
-- `odds` array (one row per outcome, not hardcoded to binary) and declares
-- an `outcome_descriptions` field in its API type, but never received real
-- data for it. This migration adds the missing columns so the indexer can
-- persist them and the API can return them.
--
-- NOTE on version numbering: this repo currently has two pairs of migration
-- files sharing the same leading version number (004_creator_incentives_...
-- / 004_enhance_schema_precision, and 009_add_predictions_indexes /
-- 009_create_user_statistics). That pre-existing collision is left alone
-- here (renumbering already-applied migrations is unsafe without access to
-- a deployed `_sqlx_migrations` history) — 012 is simply the next version
-- number that is not already in use.

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS options_count INTEGER NOT NULL DEFAULT 2,
    ADD COLUMN IF NOT EXISTS outcome_descriptions TEXT[] NOT NULL DEFAULT '{}';
