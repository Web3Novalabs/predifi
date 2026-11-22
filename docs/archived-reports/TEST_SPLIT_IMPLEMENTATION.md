# Issue #1746: Split test.rs Implementation Guide

## Overview
Split `contract/contracts/predifi-contract/src/test.rs` (13,511 lines, 308 tests) into 6 feature-based test modules, each ~1500 lines max.

## Phase 1: Extract Helper Modules ✅

Move lines 16-233 (helper modules and utilities) to a shared file.

**File to create:** `contract/contracts/predifi-contract/tests/common.rs`

Contains:
- `dummy_access_control` module (lines 16-79)
- `rogue_token` module (lines 81-158)
- Constants: `ROLE_ADMIN`, `ROLE_OPERATOR`, `ROLE_ORACLE`
- `setup()` function
- `assert_single_contract_auth()` helper function

Then update top of `src/test.rs`:
```rust
mod common;
use common::*;
```

---

## Phase 2: Create Feature Modules

### Module 1: pool_lifecycle_tests.rs (~1500 lines, 93 tests)

**Tests to extract:**
```
test_create_pool_* (30+ tests)
test_cancel_pool_* (10+ tests)
test_pool_created_* (5+ tests)
test_get_active_pools_* (10+ tests)
test_is_pool_active_* (3+ tests)
test_update_pool_description_* (4+ tests)
test_set_stake_limits_* (6+ tests)
test_pool_end_time_* (2+ tests)
test_get_pool_config_* (4+ tests)
test_get_pool_stats_* (2+ tests)
test_bump_ttl_* (4+ tests)
test_mark_pool_ready
test_outcome_descriptions_*
test_close_staking_*
test_flag_disputed_pool_*
```

**Header:**
```rust
#![cfg(test)]
use super::*;
use soroban_sdk::{...}; // same imports as original test.rs
use crate::common::*;

// actual tests here
```

---

### Module 2: admin_tests.rs (~1400 lines, 57 tests)

**Tests to extract:**
```
test_set_fee_bps_auth_only_happens_at_entry_point
test_increase_max_total_stake_auth_only_happens_at_entry_point
test_admin_can_set_* (4+ tests)
test_paused_blocks_* (12+ tests)
test_unpause_restores_functionality
test_is_contract_paused_* (5+ tests)
test_admin_can_pause_and_unpause
test_admin_can_migrate
test_init_oracle_* (2+ tests)
test_get_contract_info_*
test_admin_can_set_min_pool_duration
test_version_* (4+ tests)
test_get_fee_config_*
test_resolve_pool_auth_only_happens_at_entry_point
```

---

### Module 3: oracle_tests.rs (~1200 lines, 50 tests)

**Tests to extract:**
```
test_oracle_* (15+ tests)
test_oracle_resolve_auth_only_happens_at_entry_point
test_whitelisted_oracle_*
test_set_price_condition_* (2+ tests)
test_resolve_pool_from_price_* (3+ tests)
test_update_price_feed_* (3+ tests)
test_multi_oracle_* (3+ tests)
test_single_oracle_*
test_conflicting_oracle_votes_* (3+ tests)
test_max_price_age_*
test_double_resolution_attempt
test_set_resolution_delay_* (2+ tests)
test_already_initialized_panics
test_payout_never_exceeds_pool
test_get_fee_config_*
```

---

### Module 4: predictions_tests.rs (~1000 lines, 40 tests)

**Tests to extract:**
```
test_place_prediction_* (10+ tests)
test_max_predictions_per_* (3+ tests)
test_prediction_cooldown_* (7+ tests)
test_get_user_predictions_* (3+ tests)
test_duplicate_staking_*
test_cannot_place_prediction_*
test_get_prediction_cooldown_*
test_place_prediction_rejects_invalid_invite_key
test_paused_blocks_place_prediction
```

---

### Module 5: payouts_tests.rs (~1000 lines, 35 tests)

**Tests to extract:**
```
test_claim_winnings_* (5+ tests)
test_reward_claimed_*
test_referral_fee_distribution
test_payout_rounding_* (3+ tests)
test_large_stake_winnings_*
test_maximum_single_stake_roundtrip
test_batch_claim_winnings_*
test_claim_refund_*
test_emergency_withdraw_* (2+ tests)
test_withdraw_treasury_* (4+ tests)
test_admin_can_withdraw_treasury
test_non_admin_cannot_withdraw_treasury
test_get_fees_*
test_paused_blocks_withdraw_treasury
test_paused_blocks_batch_claim_winnings
```

---

### Module 6: whitelist_tests.rs (~900 lines, 25 tests)

**Tests to extract:**
```
test_token_whitelist_* (10+ tests)
test_add_token_to_whitelist_* (2+ tests)
test_remove_token_from_whitelist_* (2+ tests)
test_batch_add_tokens_* (2+ tests)
test_batch_remove_tokens_* (2+ tests)
test_is_whitelisted_* (3+ tests)
test_get_supported_tokens_* (4+ tests)
test_whitelist_events_*
test_whitelist_persists_*
test_delisted_token_prevents_prediction
test_place_prediction_succeeds_for_whitelisted_token
```

---

### Module 7: integration_and_utils_tests.rs (~1200 lines, 28 tests)

**Tests to extract:**
```
test_claim_winnings (the core payout test)
test_double_claim
test_claim_unresolved
test_multiple_pools_independent
test_resolution_then_new_pool_state_isolation
test_many_users_rapid_claim_after_resolution
test_events_module_publish_and_log
test_category_constants_*
test_get_pools_by_category_* (3+ tests)
test_pool_created_with_each_category
test_get_active_pools_pagination
test_pool_created_event_contains_creator
test_creator_can_cancel_empty_pool
test_any_user_can_cancel_overdue_pool
test_create_pool_accepts_metadata_url_at_512_bytes
test_create_pool_accepts_empty_metadata_url
test_cancel_pool_zero_participants_removed_from_active_index
test_cancel_pool_zero_participants_catpoolix_still_readable
test_operator_can_cancel_pool_with_bets
test_rogue_token_* (reentrancy tests)
test_version_* (misc version tests not in admin module)
etc.
```

---

## Phase 3: Update lib.rs

Remove `mod test` declaration at bottom and add:

```rust
#[cfg(test)]
mod pool_lifecycle_tests;
#[cfg(test)]
mod admin_tests;
#[cfg(test)]
mod oracle_tests;
#[cfg(test)]
mod predictions_tests;
#[cfg(test)]
mod payouts_tests;
#[cfg(test)]
mod whitelist_tests;
#[cfg(test)]
mod integration_and_utils_tests;
```

---

## Phase 4: Extraction Strategy

For each module:

1. **Identify test boundaries** using:
   ```bash
   grep -n "^fn test_" src/test.rs | grep pattern
   ```

2. **Extract tests with dependencies**:
   - Copy complete function body from `#[test]` to closing `}`
   - Include any helper functions called by tests
   - Keep docstring comments

3. **Preserve imports**:
   ```rust
   #![cfg(test)]
   use super::*;
   use soroban_sdk::{...};
   use crate::common::*;
   ```

4. **Test coverage**:
   ```bash
   cargo test --lib
   ```
   Should pass all 308 tests (no test can be lost).

5. **Size verification**:
   ```bash
   wc -l tests/*.rs
   ```
   Each file should be ~1000-1500 lines.

---

## Phase 5: Verification Checklist

- [ ] All 308 tests still pass: `cargo test --lib`
- [ ] No file exceeds ~1500 lines
- [ ] Each test appears exactly once (no duplicates, none missing)
- [ ] All imports resolve correctly
- [ ] Common module is properly referenced as `use crate::common::*;`
- [ ] Remove old `src/test.rs` after split is complete
- [ ] Verify error codes test still passes: `cargo test --test error_codes_test`

---

## Notes

- **Do not modify test logic** - preserve exact assertions and behavior
- **Helper functions** - if a test uses a helper function, move it too
- **Line count estimation** - 308 tests × ~43 lines avg = 13,244 lines (fits across 6-7 modules)
- **Module naming** - use `_tests.rs` suffix for clarity that these are test modules
- **Backwards compatibility** - tests should run identically in both old and new structure

---

## Example: Extract one test

Original in test.rs:
```rust
#[test]
fn test_create_pool_accepts_valid_referral_code() {
    let (ac_client, client, token_address, token, ...) = setup(&env);
    // test body...
}
```

In pool_lifecycle_tests.rs:
```rust
#[test]
fn test_create_pool_accepts_valid_referral_code() {
    let (ac_client, client, token_address, token, ...) = setup(&env);
    // test body... (identical)
}
```

The setup() and other common items come from `use crate::common::*;`
