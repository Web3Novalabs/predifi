# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
