# Issue 352: Automated State Cleanup and Garbage Collection Logic

## Description

Implement a routine to prune old, resolved market data from "active" storage indexes. This keeps storage costs low and prevents the state from bloating over time.

## Tasks

- Implement a `prune_resolved_pools` function.
- Define data rent recovery strategies (claiming back XLM from expired storage).
- Automate cleanup triggers during low-activity periods or batch resolutions.

## Dependencies

- Issue #302
- Issue #333
- Issue #347
