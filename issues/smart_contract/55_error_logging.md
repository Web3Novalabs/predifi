# Issue 354: Implement Detailed Error Logs for Frontend Integration

## Description

While Issue #301 handled error types, this issue focuses on ensuring that every error path emits a rich event containing debug information (caller, arguments, state flags) to help the frontend explain failures to the user.

## Tasks

- Define a standard `ContractErrorEvent`.
- Map all `Err` returns to an event emission before failing.
- Ensure sensitive data (private keys/links) is NOT logged.

## Dependencies

- Issue #301
- Issue #325
