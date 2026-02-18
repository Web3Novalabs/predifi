# Issue 302: Optimize Storage Usage Patterns (Ttl, Instance vs Persistent)

## Description

Review the current usage of `env.storage().instance()`. Move data that grows linearly (like individual user predictions) to `persistent` storage and set appropriate TTLs to manage contract state efficiency and costs.

## Tasks

- Identify data keys suitable for `persistent` storage.
- Update `lib.rs` to use `persistent()` for user-specific data.
- Standardize TTL extension logic for long-lived markets.

## Dependencies

- Issue #300
