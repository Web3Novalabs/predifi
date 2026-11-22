# Reentrancy Protection - Visual Reference Guide

## 1. Claim Flow Diagram

### claim_winnings_internal() - Complete Control Flow

```
                        ┌─────────────────────────────────┐
                        │ claim_winnings_internal()       │
                        │ (Lines 3425-3595)               │
                        └──────────────┬──────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────┐
                        │ ENTER REENTRANCY GUARD   │
                        │ (Line 3440)              │
                        │ panic!("Reentrancy")     │
                        └──────────────┬───────────┘
                                       │
                    ┌──────────────────┴──────────────────┐
                    │ CHECKS PHASE (Lines 3443-3475)      │
                    │ ✓ Load pool from storage            │
                    │ ✓ Verify pool != Active             │
                    │ ✓ Check !AlreadyClaimed ← CRITICAL  │
                    │ ✓ Load user prediction              │
                    │ ✓ Validate prediction exists        │
                    └──────────────────┬───────────────────┘
                                       │
                                       ▼
                    ┌──────────────────────────────────────┐
                    │ EFFECTS PHASE (Lines 3472-3476)      │
                    │ ⚡ SET Claimed = true ← STATE LOCKED  │
                    │ ⚡ Bump TTL on storage keys           │
                    └──────────────────┬───────────────────┘
                                       │
                    ┌──────────────────┴───────────────────┐
                    │                                      │
                    ▼                                      ▼
        ┌──────────────────────┐          ┌──────────────────────┐
        │ CANCELED POOL PATH   │          │ RESOLVED POOL PATH   │
        │ (Line 3481-3493)     │          │ (Line 3496-3585)     │
        │                      │          │                      │
        │ Transfer refund      │          │ Calculate fees       │
        │ amount to user       │          │ Calculate winnings   │
        │ Emit events          │          │ Check referrer       │
        └──────────────────────┘          │ Transfer referral    │
                 │                        │ Transfer winnings    │
                 │                        │ Emit events          │
                 └────────────┬───────────┘
                              │
                    ┌─────────▼──────────┐
                    │ INTERACTIONS PHASE │
                    │ All transfers      │
                    │ state LOCKED ✓     │
                    └─────────┬──────────┘
                              │
                    ┌─────────▼──────────────────┐
                    │ EXIT REENTRANCY GUARD      │
                    │ (Line 3595)                │
                    │ Remove temporary storage   │
                    └─────────┬──────────────────┘
                              │
                    ┌─────────▼──────────────────┐
                    │ RETURN Result              │
                    │ ✅ Success or ❌ Error     │
                    └────────────────────────────┘
```

---

## 2. Reentry Attack Prevention Diagram

### What Happens on Reentry Attempt?

```
NORMAL EXECUTION:
    ┌────────────────────────┐
    │ First call to          │
    │ claim_winnings()       │
    └────────┬───────────────┘
             │
             ▼
    ┌────────────────────────────────────┐
    │ enter_reentrancy_guard()           │
    │ Set temporary storage key          │
    │ ✓ Key: DataKey::RentGuard = true   │
    └────────┬───────────────────────────┘
             │
             ▼
    ┌────────────────────────────┐
    │ Execute claim logic        │
    │ Set Claimed flag           │
    └────────┬───────────────────┘
             │
             ▼
    ┌────────────────────────────────────┐
    │ Token transfer executes            │
    │ Transfer function called on token  │
    └────────────────────────────────────┘
             │
             ▼
    ┌────────────────────────────────────┐
    │ REENTRY ATTACK ATTEMPT!            │
    │ Malicious code calls claim_winnings│
    │ again DURING token transfer        │
    └────────┬───────────────────────────┘
             │
             ▼
    ┌────────────────────────────────────────────┐
    │ enter_reentrancy_guard() called again      │
    │ Check: env.storage().temporary()           │
    │        .has(&DataKey::RentGuard)           │
    │ Result: ✓ KEY EXISTS!                      │
    └────────┬─────────────────────────────────┘
             │
             ▼
    ┌────────────────────────────────────┐
    │ 🔴 PANIC!                          │
    │ "Reentrancy detected"              │
    │ Transaction REVERTS                │
    │ No state change                    │
    │ No tokens transferred              │
    └────────────────────────────────────┘
```

---

## 3. Protection Layers - Defense in Depth

### Three-Layer Defense System

```
┌──────────────────────────────────────────────────────────────────┐
│                    REENTRANCY ATTACK VECTOR                       │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
    ┌────────────────────────────┐
    │  LAYER 1: GUARD MUTEX      │
    │  Type: Temporary storage   │
    │  Scope: Transaction only   │
    │  Effect: Hard panic        │
    │  ❌ Blocks attack          │
    │  (Lines 1309-1320)         │
    └────────────┬───────────────┘
                 │
         LAYER 1 PASSES ✓
         (Happens if code calls
          same function from different
          transaction context)
                 │
                 ▼
    ┌────────────────────────────┐
    │ LAYER 2: CLAIMED FLAG      │
    │ Type: Persistent storage   │
    │ Scope: Per-user per-pool   │
    │ Effect: Error return       │
    │ ❌ Blocks claim            │
    │ (Lines 3460, 3472, 3697)   │
    └────────────┬───────────────┘
                 │
         LAYER 2 PASSES ✓
         (Somehow flag not set
          or second pool)
                 │
                 ▼
    ┌────────────────────────────┐
    │ LAYER 3: CEI PATTERN       │
    │ Type: Code ordering        │
    │ Scope: Effects before I/O  │
    │ Effect: State locked       │
    │ ❌ No re-read possible     │
    │ (Lines 3472→3484, 3704→3717)
    └────────────────────────────┘

RESULT: No viable attack path remains ✅
```

---

## 4. Claimed Flag State Machine

### State Transitions Diagram

```
                    INITIAL STATE
                    (not claimed)
                          │
                          │
                          ▼
             ┌─────────────────────────┐
             │ claim_winnings() called  │
             │ or claim_refund() called │
             └────────────┬────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │ Check phase: │
                   │ has(Claimed) │
                   └──────┬───────┘
                          │
           ┌──────────────┼──────────────┐
           │              │              │
    ❌ YES (Flag      🟢 NO (First  (impossible)
      exists)          time)
           │              │
           ▼              ▼
    ┌────────────┐  ┌─────────────────┐
    │ Return     │  │ EFFECTS:        │
    │ AlreadyClaimed
    │   error    │  │ env.storage()   │
    │ ✓ Blocked  │  │ .set(           │
    └────────────┘  │   Claimed=true  │
                    │ )               │
                    └────────┬────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ FINAL STATE:     │
                    │ (claimed=true)   │
                    │ Permanent ✓      │
                    └──────────────────┘
```

---

## 5. Transaction Execution Timeline

### With and Without Reentrancy

```
SAFE EXECUTION (CEI Pattern):
┌─────────────────────────────────────────────────┐
│ Time                                            │
│  │                                             │
│  ├─ 0ms ─► enter_reentrancy_guard()           │
│  │         ↳ Guard set in temp storage        │
│  │                                             │
│  ├─ 1ms ─► CHECKS (validate inputs)           │
│  │                                             │
│  ├─ 2ms ─► EFFECTS (update state)             │
│  │         ↳ Claimed flag set                 │
│  │         ↳ State now IMMUTABLE              │
│  │                                             │
│  ├─ 3ms ─► INTERACTIONS (transfer tokens)     │
│  │         ↳ state cannot change              │
│  │         ↳ transfer succeeds                │
│  │                                             │
│  ├─ 4ms ─► exit_reentrancy_guard()            │
│  │         ↳ Guard removed                    │
│  │                                             │
│  └─ 5ms ─► Return success                     │
│                                                │
│ Transaction COMMITTED ✅                       │
└─────────────────────────────────────────────────┘


ATTACK ATTEMPT (During Transfer):
┌──────────────────────────────────────────────────┐
│ Time                                             │
│  │                                              │
│  ├─ 0ms ─► enter_reentrancy_guard()            │
│  │         ↳ Guard set                         │
│  │                                              │
│  ├─ 1ms ─► CHECKS                              │
│  │                                              │
│  ├─ 2ms ─► EFFECTS (Claimed = true)            │
│  │                                              │
│  ├─ 3ms ─► token.transfer() starts             │
│  │                                              │
│  │◄──────── ATTACK: reentrant call to          │
│  │          claim_winnings()                    │
│  │                                              │
│  ├─ 3.5ms ─► enter_reentrancy_guard()          │
│  │           ↳ Check: guard exists? ✓ YES     │
│  │           ↳ PANIC! ❌                       │
│  │           ↳ Transaction REVERTED            │
│  │                                              │
│  └─ 4ms ─► Transaction ABORTED ❌              │
│                                                 │
│ State rolled back, no damage ✅                 │
└──────────────────────────────────────────────────┘
```

---

## 6. Storage State After Claim

### Persistent Storage Changes

```
BEFORE claim_winnings():
┌──────────────────────────────────────┐
│ Persistent Storage                   │
│                                      │
│ Predicted(user, pool_id) = {        │
│   amount: 100 USDC                  │
│   outcome: 2                         │
│ }                                    │
│                                      │
│ (Claimed key DOES NOT EXIST)        │
│                                      │
└──────────────────────────────────────┘


AFTER claim_winnings() succeeds:
┌──────────────────────────────────────┐
│ Persistent Storage                   │
│                                      │
│ Predicted(user, pool_id) = {        │
│   amount: 100 USDC                  │
│   outcome: 2                         │
│ }  (unchanged)                       │
│                                      │
│ Claimed(user, pool_id) = true ✓     │
│ ← NEW KEY ADDED (write-once)        │
│                                      │
│ Pool.total_stake decreased ✓         │
│ (after winnings transferred)         │
│                                      │
└──────────────────────────────────────┘


ATTEMPT claim_winnings() again:
┌──────────────────────────────────────┐
│ First check in claim_winnings():     │
│                                      │
│ if has(Claimed(user, pool)) {       │
│   ❌ KEY EXISTS                      │
│   return AlreadyClaimed error        │
│ }                                    │
│                                      │
│ Result: Early return, no transfers   │
│ State unchanged ✓                    │
│                                      │
└──────────────────────────────────────┘
```

---

## 7. Comparison: Vulnerable vs. Protected

### Vulnerable Pattern (Anti-Pattern)

```
fn vulnerable_claim() {
    // ❌ BAD: Effects AFTER interactions
    
    let amount = calculate_winnings();
    
    // ❌ VULNERABLE: Transfer before state update
    token.transfer(user, amount);
    
    // ❌ TOO LATE: Flag set after transfer
    storage.set(claimed, true);
    
    // If token transfer calls back into contract:
    // 1. Can call claim_winnings again
    // 2. Claimed flag not yet set
    // 3. Calculate same amount again
    // 4. Transfer again
    // 5. User gets double payment ❌
}
```

### Protected Pattern (Correct)

```
fn protected_claim() {
    // ✅ GOOD: CEI ordering
    
    guard.enter();  // Layer 1: Guard
    
    try {
        // CHECKS
        validate_pool();
        if claimed_flag_exists() {  // Layer 2: Flag
            return error;  // Prevent reentry
        }
        
        // EFFECTS (before external calls)
        set(claimed, true);  // State locked
        amount = calculate_winnings();
        
        // INTERACTIONS (after state locked)
        token.transfer(user, amount);
        
        // If token transfer calls back:
        // 1. Reentrancy call attempts claim
        // 2. Guard.enter() finds guard exists
        // 3. PANIC ❌ Reentry blocked
        // 4. No double payment ✓
        
    } finally {
        guard.exit();  // Layer 1: Release guard
    }
}
```

---

## 8. Attack Vector Coverage Matrix

### Which Attack Is Blocked By Which Layer?

```
┌─────────────────────────┬───────────┬───────────┬─────────┐
│ Attack Type             │ Guard L1  │ Flag L2   │ CEI L3  │
├─────────────────────────┼───────────┼───────────┼─────────┤
│ Simple fallback         │ ✅ PANIC  │ ✅ Check  │ ✅ Lock │
│ ERC-777 hook            │ ✅ PANIC  │ ─         │ ─       │
│ Flash loan              │ ─         │ ✅ Check  │ ✅ Lock │
│ Cross-function call     │ ✅ PANIC  │ ✅ Check  │ ✅ Lock │
│ Batch duplicates        │ ─         │ ✅ Check  │ ✅ Lock │
│ Direct state mutation   │ ─         │ ─         │ ✅ Lock │
└─────────────────────────┴───────────┴───────────┴─────────┘

Multiple vectors blocked by multiple layers = Defense in Depth ✅
```

---

## 9. Code Snippet Reference

### Key Protection Points - 30-Second Summary

```rust
// ⭐ LAYER 1: Guard (Lines 1309-1320)
fn enter_reentrancy_guard(env: &Env) {
    if env.storage().temporary().has(&DataKey::RentGuard) {
        panic!("Reentrancy detected");  // ← Hard stop
    }
    env.storage().temporary().set(&DataKey::RentGuard, &true);
}

// ⭐ LAYER 2: Claimed Flag Check (Line 3460)
let claimed_key = DataKey::Claimed(user.clone(), pool_id);
if env.storage().persistent().has(&claimed_key) {
    return Err(PredifiError::AlreadyClaimed);  // ← Double-claim block
}

// ⭐ LAYER 3: CEI - Effects Before Interactions
env.storage().persistent().set(&claimed_key, &true);  // (line 3472)
Self::bump_ttl(env, &claimed_key);                     // State locked!

token_client.transfer(...);  // (line 3484) - Safe to transfer now
```

---

## 10. Quick Checklist - Is This Call Safe?

### Verification Flowchart

```
                    Examining: claim_winnings()?
                            │
                            ▼
                   Has reentrancy guard?
                      /             \
                    YES              NO
                    │                 └──► 🔴 UNSAFE
                    │
                    ▼
           Guard entered at function start?
                 /              \
               YES              NO
               │                 └──► 🔴 UNSAFE
               │
               ▼
          Guard exited at function end?
             /             \
           YES              NO
           │                 └──► 🔴 UNSAFE (guard might not release)
           │
           ▼
    Checks before effects?
        /              \
      YES              NO
      │                 └──► 🔴 UNSAFE (wrong order)
      │
      ▼
   State updated before transfers?
       /               \
     YES               NO
     │                  └──► 🔴 UNSAFE (vulnerable to reentry)
     │
     ▼
  Double-claim flag set?
      /             \
    YES              NO
    │                 └──► 🔴 UNSAFE (can claim twice)
    │
    ▼
🟢 SAFE - All layers present and correct! ✅
```

---

## 11. Event Emission Timeline

### What Events Fire During Normal Claim?

```
User calls claim_winnings()
           │
           ▼
       Validate checks
           │
           ▼
       Set Claimed flag
           │
           ▼
       Calculate amount
           │
           ▼
       Transfer tokens
           │
           ▼
    📤 WinningsClaimedEvent emitted
    │  ├─ pool_id
    │  ├─ user
    │  └─ amount
    │
    ▼
    📤 RewardClaimedEvent emitted
    │  ├─ pool_id
    │  ├─ user
    │  ├─ amount
    │  └─ claim_type: "winnings"
    │
    ▼
   Exit guard
    │
    ▼
   Return success


On double-claim attempt:
           │
           ▼
       Check Claimed flag exists? YES
           │
           ▼
    📤 SuspiciousDoubleClaimEvent emitted ⚠️
    │  ├─ user
    │  ├─ pool_id
    │  └─ timestamp
    │
    ▼
    Return AlreadyClaimed error
    (No funds transferred)
```

---

**Use this visual guide to quickly understand the reentrancy protections in place.**

All diagrams reference exact line numbers in the code for verification.
