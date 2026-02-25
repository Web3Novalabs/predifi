# Predifi Contract Performance Benchmarks

This document outlines the performance characteristics, storage costs, and theoretical limits of the Predifi contract.

## Storage Costs per Operation

Soroban storage costs are primarily driven by the number of persistent entries and their size.

| Operation | Storage Entries Touched | Cost Type | Details |
| :--- | :--- | :--- | :--- |
| `create_pool` | 4 persistent, 1 instance | High | `Pool`, `PoolIdCounter`, `CategoryPoolCount`, `CategoryPoolIndex` |
| `place_prediction` | 4-5 persistent | Medium | `Prediction`, `Pool` (total stake), `OutcomeStakes` (batch), `UserPredictionCount`, `UserPredictionIndex` |
| `resolve_pool` | 1 persistent | Low | `Pool` (state update) |
| `claim_winnings` | 1 persistent | Low | `HasClaimed` marker |

## Theoretical Limits

Based on current Soroban ledger limits (Standard/Medium network settings):

### 1. Maximum Number of Outcomes
The contract uses an optimized batch storage pattern for `OutcomeStakes`.
- **Max Outcomes**: ~8,000 (limited by 64KB per persistent entry).
- **Recommended**: 2 to 100 for optimal performance and indexing.

### 2. Maximum Predictions per Pool
Predictions are stored as individual persistent entries.
- **Limit**: Millions (limited by ledger capacity and user storage limits).
- **Scale**: The contract uses `DataKey::Prediction(User, PoolId)`, allowing O(1) lookup for individual claims.

### 3. Payout Throughput
- **Resolution**: O(1) complexity (resolves the entire pool in one transaction).
- **Claiming**: O(1) complexity per user. Scalable across multiple users/transactions.

## Stress Test Results (Local Mock)

| Test Case | Volume | Status | Result |
| :--- | :--- | :--- | :--- |
| High Volume Predictions | 100 users / single pool | PASS | Successful stake aggregation |
| Bulk Claim Winnings | 48 winners | PASS | Isolation and prevention of double claims |
| Sequential Pool Creation | 50 pools | PASS | Linear storage growth |
| Throughput Measurement | 50 consecutive predictions | PASS | Consistent latency |
| Multi-pool Resolution | 10 pools simultaneously | PASS | Proper resolution isolation |

## Optimization Recommendations
1.  **State Archival**: Frequent players should be aware that `Prediction` entries are persistent and may require TTL extension if dormant for > 6 months.
2.  **Batch Indexing**: The `get_user_predictions` function is paginated to handle users with 1000+ predictions without hitting resource limits.
