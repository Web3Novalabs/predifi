# Issue 344: Implement Maintenance Mode for Specific Categories

## Description

Extend the global circuit breaker to support category-specific pauses. This allows the admin to halt "Sports" markets if an API is down, without affecting "Finance" or "Crypto" markets.

## Tasks

- Update `Paused` state to include a map of Categories to their status.
- Allow the admin to toggle specific categories.
- Update `create_pool` and `place_prediction` to check category-specific pauses.

## Dependencies

- Issue #306
- Issue #331
- Issue #302
