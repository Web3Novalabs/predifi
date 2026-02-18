# Issue 319: Add Treasury Withdrawal Functionality

## Description

Currently, fees are sent to the treasury address upon the first claim. Implement a more flexible mechanism for the admin to withdraw accumulated protocol fees or unused liquidity from the treasury-related accounts.

## Tasks

- Implement `withdraw_treasury` function for Admin only.
- Support withdrawals of multiple tokens.
- Add audit log events for all treasury transfers.

## Dependencies

- Issue #303
- Issue #312
