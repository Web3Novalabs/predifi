# Security Audit: Access Control for Admin Functions

**Issue:** #1918 (supersedes #1351)  
**Scope:** `contract/contracts/predifi-contract/src/` (`admin.rs`, `treasury.rs`, `oracle.rs`, `referral.rs`, `lib.rs`)

## Summary

All admin-restricted and configuration setter functions strictly enforce host-managed cryptographic authentication via `admin.require_auth()` followed by role assertion via `Self::require_admin_role()` before any state mutation, storage write, or token transfer. The execution pattern is verified consistent across every privileged setter and operational entry point.

## Audited Functions Matrix

| Function | File Location | `require_auth()` | `require_admin_role()` | State Mutation Guarded? |
|---|---|:---:|:---:|:---:|
| `pause` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `unpause` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_fee_bps` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `apply_fee_bps` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `cancel_fee_proposal` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_treasury` | `src/treasury.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `withdraw_treasury` | `src/treasury.rs` | ✅ | ✅ (Role 0) | ✅ Before transfer |
| `emergency_withdraw` | `src/treasury.rs` | ✅ | ✅ (Role 0) | ✅ Before transfer |
| `add_oracle` | `src/oracle.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `remove_oracle` | `src/oracle.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `init_oracle` | `src/oracle.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `upgrade_contract` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before upgrade |
| `migrate_state` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_max_predictions_per_user` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_prediction_cooldown` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_resolution_delay` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_claim_window` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_min_pool_duration` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_min_stake` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `add_token_to_whitelist` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `remove_token_from_whitelist` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `batch_add_tokens_to_whitelist` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `batch_remove_tokens_whitelist` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_fee_tiers` | `src/admin.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_referral_cut_bps` | `src/referral.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_referral_rate` | `src/referral.rs` | ✅ | ✅ (Role 0) | ✅ Before write |
| `set_referral_volume_threshold` | `src/referral.rs` | ✅ | ✅ (Role 0) | ✅ Before write |

Operator-scoped functions (`set_stake_limits`, `set_price_condition`) use
`operator.require_auth()` + `require_role(&env, &operator, 1)` — appropriate
for Role::Operator (role id 1).

## Auth Pattern & Execution Sequencing

Every admin and privileged function strictly follows this clean architecture order:

```rust
// 1. Precondition & pause checks (pure queries, zero mutations)
Self::require_not_paused(&env)?;

// 2. Soroban host cryptographic signature verification
admin.require_auth();

// 3. Role-based access control cross-contract verification
Self::require_admin_role(&env, &admin, "fn_name")?;

// 4. Parameter bound validation (e.g., fee_bps <= 10_000, amounts > 0)
// ...

// 5. State mutations only after all checks pass
env.storage().instance().set(&DataKey::Config, &config);
Self::extend_instance(&env);

// 6. Audit event emission
AdminOpEvent { admin, ... }.publish(&env);
```

`require_admin_role` delegates to `require_role(env, admin, 0)` which calls
the external `access-control` contract to assert Role::Admin (role id 0).
Unauthorized calls emit an `UnauthorizedAdminOpEvent` and return
`PredifiError::Unauthorized`.

## Time & Space Complexity

- **Authorization Checking**: $\mathcal{O}(1)$ time complexity via host VM cryptographic signature verification and cross-contract role lookup.
- **Space Complexity**: $\mathcal{O}(1)$ auxiliary space overhead.

## Findings

No missing authorization checks were identified. All admin and setter functions properly
guard state mutations with both Soroban-level authentication (`admin.require_auth()`) and role-based
access control before any storage writes or token transfers. Full automated regression coverage is verified in `access_control_audit_tests.rs`.
