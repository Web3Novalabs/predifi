# Issue 313: Design Oracle Callback Interface

## Description

Design a standardized interface for external oracles to push resolution data to PrediFi. This interface should allow oracles to provide the winning outcome and a proof or reference ID for the event.

## Tasks

- Define a trait or standardized function signature for oracle callbacks.
- Implement an authorized `oracle_resolve` function.
- Add security checks to ensure only registered oracles can call it.

## Dependencies

- Issue #303
- Issue #308
