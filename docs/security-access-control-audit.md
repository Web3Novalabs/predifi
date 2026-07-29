# Security Audit: Access Control for Admin Functions

**Issue:** #1351  
**Scope:** `contract/contracts/predifi-contract/src/lib.rs`

## Summary

All admin-restricted functions call `admin.require_auth()` followed by
`Self::require_admin_role()` before any state mutation. The pattern is
consistent across every setter and privileged operation.

## Audited Functions

| Function | `require_auth()` | `require_admin_role()` | Before state mutation |
|---|---|---|---|
| `pause` | ✅ | ✅ | ✅ |
| `unpause` | ✅ | ✅ | ✅ |
| `set_fee_bps` | ✅ | ✅ | ✅ |
| `set_treasury` | ✅ | ✅ | ✅ |
| `set_max_predictions_per_user` | ✅ | ✅ | ✅ |
| `set_prediction_cooldown` | ✅ | ✅ | ✅ |
| `set_resolution_delay` | ✅ | ✅ | ✅ |
| `set_claim_window` | ✅ | ✅ | ✅ |
| `set_min_pool_duration` | ✅ | ✅ | ✅ |
| `set_min_stake` | ✅ | ✅ | ✅ |
| `set_referral_cut_bps` | ✅ | ✅ | ✅ |
| `set_referral_rate` | ✅ | ✅ | ✅ |
| `set_fee_tiers` | ✅ | ✅ | ✅ |
| `set_referral_volume_threshold` | ✅ | ✅ | ✅ |
| `add_oracle` | ✅ | ✅ | ✅ |
| `remove_oracle` | ✅ | ✅ | ✅ |
| `upgrade_contract` | ✅ | ✅ | ✅ |
| `withdraw_treasury` | ✅ | ✅ | ✅ |

Operator-scoped functions (`set_stake_limits`, `set_price_condition`) use
`operator.require_auth()` + `require_role(&env, &operator, 1)` — appropriate
for Role::Operator (role id 1).

## Auth Pattern

Every admin function follows this order:

```rust
admin.require_auth();                                      // 1. Soroban auth check
Self::require_admin_role(&env, &admin, "fn_name")?;        // 2. Role-based ACL
// ... state mutations only after both checks pass
```

`require_admin_role` delegates to `require_role(env, admin, 0)` which calls
the external `access-control` contract to assert Role::Admin (role id 0).
Unauthorized calls emit an `UnauthorizedAdminOpEvent` and return
`PredifiError::Unauthorized`.

## Findings

No missing authorization checks were identified. All admin functions properly
guard state mutations with both Soroban-level authentication and role-based
access control before any storage writes.
