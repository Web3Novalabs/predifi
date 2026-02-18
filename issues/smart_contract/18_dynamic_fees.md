# Issue 317: Support Dynamic Fee Tiers Based on Volume

## Description

Implement a dynamic fee system where the protocol fee percentage decreases as the total pool stake increases. This encourages participation in larger markets.

## Tasks

- Create a fee tier structure (e.g., Stake > 1M XLM = 0.5% fee).
- Update `resolve_pool` to calculate fees based on the final total stake and applicable tiers.
- Add admin functions to update fee tier thresholds.

## Dependencies

- Issue #304
- Issue #303
