# Issue 323: Add Method to Query Detailed Pool Statistics

## Description

Implement a read-only function that returns a comprehensive view of a pool's state, including total stake per outcome, number of participants, and current implied odds.

## Tasks

- Design a `PoolStats` struct.
- Implement `get_pool_stats` function.
- Ensure efficient data aggregation to stay within Soroban's read limits.

## Dependencies

- Issue #302
- Issue #305
- Issue #321
