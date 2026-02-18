# Issue 358: Rate Limiting and Anti-Spam Protection Logic

## Description

Protect the protocol from spam market creation or prediction floods that could bloat storage or degrade performance for others.

## Tasks

- Implement a cooldown period for market creation per user.
- Add adjustable "spam" fees for rapid-fire predictions.
- Create on-chain counters for spam detection.

## Dependencies

- Issue #333
- Issue #302
- Issue #346
