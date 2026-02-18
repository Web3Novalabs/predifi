# Issue 327: Increase Unit Test Coverage for Edge Cases

## Description

Expand the current `test.rs` to cover complex edge cases such as leap-year timestamps, maximum possible stakes, and rapid resolution/claim sequences.

## Tasks

- Add tests for boundary values in all validation logic.
- Simulate race conditions and unauthorized access attempts.
- Verify state consistency after multiple resolution cycles.

## Dependencies

- Issue #300
- Issue #301
