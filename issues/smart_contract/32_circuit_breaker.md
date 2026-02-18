# Issue 331: Implement Emergency Global Circuit Breaker

## Description

Implement a "pause" mechanism that allowed an administrator to halt all betting and resolving activities in case of a critical vulnerability or network-level emergency.

## Tasks

- Add a `paused` state to the contract instance storage.
- Implement `pause` and `unpause` functions with Admin authorization.
- Update all state-changing functions to check the `paused` flag before execution.

## Dependencies

- Issue #303
