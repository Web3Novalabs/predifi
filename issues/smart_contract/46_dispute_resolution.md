# Issue 345: Decentralized Dispute Resolution Mechanism

## Description

Implement a mechanism for users to challenge a market's resolution. This requires a "challenge period," a "bond" (stake needed to challenge), and a secondary voting or oracle escalation process.

## Tasks

- Add a `Disputed` state to markets.
- Implement `challenge_resolution` function with a bond requirement.
- Logic to integrate with a secondary, higher-tier oracle or community vote for the final outcome.

## Dependencies

- Issue #313
- Issue #309
- Issue #330
