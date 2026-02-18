# Issue 308: Implement Minimum Resolution Window Delay

## Description

To prevent front-running or premature resolution, implement a mandatory delay between the pool end time and the earliest possible resolution time. This allows time for dispute or verification of the external event.

## Tasks

- Add a configurable global resolution delay parameter.
- Update `resolve_pool` to check against this delay.
- Emit an event when a pool becomes ready for resolution.

## Dependencies

- Issue #303
- Issue #307
