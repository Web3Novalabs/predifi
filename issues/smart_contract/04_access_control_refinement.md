# Issue 303: Refine Role-Based Access Control Patterns

## Description

Harden the integration with the `access_control` contract. Ensure that critical administrative functions (e.g., `set_fee_bps`, `set_treasury`) are strictly protected and that the contract can handle role updates correctly.

## Tasks

- Verify `require_auth` usage in all admin functions.
- Ensure the `Operator` role is correctly utilized for oracles.
- Add tests for unauthorized access attempts.

## Dependencies

- Issue #300
