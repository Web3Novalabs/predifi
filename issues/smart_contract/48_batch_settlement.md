# Issue 347: Batch Settlement for High-Volume Markets

## Description

Allow an administrator or the protocol to resolve multiple markets in a single transaction. This is critical for efficiency when many small events end at the same time (e.g., end of a sports match day).

## Tasks

- Implement `batch_resolve` function.
- Add loop protection to stay within Soroban's transaction resource limits.
- Ensure atomic failure (if one market in the batch fails, others should still be resolvable later).

## Dependencies

- Issue #313
- Issue #333
