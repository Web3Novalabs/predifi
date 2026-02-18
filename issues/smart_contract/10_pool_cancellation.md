# Issue 309: Implement Pool Cancellation Logic (Admin/Creator)

## Description

Allow an administrator or the pool creator (under certain conditions) to cancel a pool if the event is voided or errors occurred during setup. This should freeze all betting and enable the refund process.

## Tasks

- Add `cancel_pool` function with authorization checks.
- Update pool state to include a `Canceled` status.
- Ensure resolved pools cannot be canceled.

## Dependencies

- Issue #303
- Issue #305
