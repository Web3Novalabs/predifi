# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `update_price_feed` now rejects an update whose `timestamp` is not strictly newer than the one
  already stored for the same `feed_pair` (`PredifiError::InvalidData`), so a delayed or replayed
  submission can no longer overwrite a newer price (#2018).
- `oracle_resolve` and `resolve_pool_from_price` return `PoolNotFound` / `PriceFeedNotFound`
  instead of aborting with a VM panic when the pool or feed does not exist; no error codes changed
  (#2019).
- Re-indented the module docs in `price_feed.rs` so Clippy's `doc_overindented_list_items` and
  `doc_lazy_continuation` lints pass (#2015).

### Added
- Pool creator incentive system (#1933): `claim_creator_incentive` pays a
  pool's creator a configurable share (`set_creator_incentive_bps`) of that
  pool's protocol fee once it reaches a minimum participation threshold
  (`set_creator_min_participants`, default 5 unique participants) and is
  resolved. Reward/reputation metrics are queryable via `get_creator_stats`.

## [0.1.0] - 2026-03-24

### Added
- `create_pool`: Create a new prediction pool with configurable parameters (end time, token, options, etc.).
- `place_prediction`: Allow users to stake tokens on a specific outcome.
- `resolve_pool`: Administrative resolution of a pool.
- `oracle_resolve`: Oracle-based resolution of a pool with multiple required votes.
- `claim_winnings`: Allow winners to claim their share of the total stake.
- `cancel_pool`: Administrative cancellation of a pool.
- `claim_refund`: Allow participants to reclaim stakes from canceled pools.
- Protocol configuration for fees, treasury, and access control.
- Token whitelisting for allowed betting assets.
- High-value prediction monitoring and alert events.
- Market categories (Sports, Finance, Crypto, Politics, Entertain, Tech, Other).

[Unreleased]: https://github.com/Dami24-hub/predifi/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Dami24-hub/predifi/releases/tag/v0.1.0
