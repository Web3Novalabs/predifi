# Treasury Withdrawal Authority & Limits

This document describes who may withdraw funds from the PrediFi protocol
treasury, what limits apply, and what happens if the withdrawing authority
becomes unavailable.

## Authority

All treasury withdrawal functions are restricted to the **Admin** — the
account holding **role 0** in the companion access-control contract.

The Admin must satisfy two checks before any withdrawal is executed:

1. `admin.require_auth()` — a valid Soroban authentication signature.
2. `Self::require_admin_role(&env, &admin, "fn_name")` — a cross-contract
   call to the access-control contract asserting role 0.

Unauthorised calls emit an `UnauthorizedAdminAttemptEvent` and return
`PredifiError::Unauthorized`.

The Admin may perform the following treasury operations:

| Function | Purpose |
|---|---|
| `set_treasury` | Repoint the treasury recipient address |
| `withdraw_treasury` | Sweep accrued protocol fees (or any unused liquidity) to a recipient |
| `emergency_withdraw` | Rescue any token balance from the contract (bypasses pause) |

## Limits

### Per-call limit

| Parameter | Value |
|---|---|
| Minimum withdrawal amount | `MIN_WITHDRAWAL_AMOUNT` = **1** (base token units / stroops) |
| Maximum withdrawal amount | The contract's current token balance for the requested token |

Withdrawals of zero or negative amount are rejected with `InvalidAmount`.
Requests exceeding the contract balance are rejected with `InsufficientBalance`.

### Per-period limit

**There is no per-period limit.** An admin may call `withdraw_treasury` any
number of times per ledger close without a cooldown, rate-limit, or cap.

## Unavailable Authority

The contract does **not** implement a multi-signature scheme, a timelock,
or any on-chain recovery mechanism that bypasses the Admin role.

If the Admin private key is lost or the Admin is otherwise unavailable:

- **Funds already withdrawn** to the configured treasury address remain
  accessible to whoever controls that address.
- **Funds still in the contract** cannot be recovered — both
  `withdraw_treasury` and `emergency_withdraw` require the Admin role, and
  there is no fallback path.
- **Pausing the contract** does not help: `emergency_withdraw` bypasses the
  pause check but still requires the Admin role.

Operators should secure the Admin key off-chain (e.g. multi-sig, hardware
wallet, or a well-rehearsed key-recovery process) because the contract
provides no on-chain mechanism to replace or recover the Admin.
