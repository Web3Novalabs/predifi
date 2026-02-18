# Issue 310: Develop Refund Mechanism for Canceled Pools

## Description

Once a pool is canceled, users should be able to reclaim their full stake without protocol fees. This requires a dedicated refund function that verifies the pool status and ensures each user only claims their original amount once.

## Tasks

- Implement `claim_refund` function.
- Verify `Canceled` status before allowing refunds.
- Use `HasClaimed` data key to prevent double refunds.

## Dependencies

- Issue #309
- Issue #302
