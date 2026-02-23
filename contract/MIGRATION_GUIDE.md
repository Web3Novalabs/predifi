# Migration Guide - String to Symbol Categories

## Overview

This guide helps you migrate from the old String-based category system to the new Symbol-based system.

## Migration Options

### Option 1: Fresh Deployment (Recommended)

**Best for:** New projects or projects in early testing phase

**Steps:**
1. Deploy the new contract version
2. Update all client integrations
3. No data migration needed

**Pros:**
- Clean slate
- No migration complexity
- Immediate benefits

**Cons:**
- Loses historical data
- Requires new contract address

### Option 2: Data Migration

**Best for:** Production contracts with active pools

**Steps:**
1. Deploy migration contract
2. Read existing pools
3. Add default categories
4. Rebuild indices
5. Update contract reference

**Pros:**
- Preserves historical data
- Maintains contract continuity

**Cons:**
- Complex migration process
- Requires careful testing
- Potential downtime

## Migration Script Template

```rust
// migration_contract.rs
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec};

#[contract]
pub struct MigrationContract;

#[contractimpl]
impl MigrationContract {
    /// Migrate pools from old contract to new contract
    /// 
    /// This function:
    /// 1. Reads all pools from old contract
    /// 2. Assigns default category (CATEGORY_OTHER)
    /// 3. Creates pools in new contract
    /// 4. Rebuilds category indices
    pub fn migrate_pools(
        env: Env,
        old_contract: Address,
        new_contract: Address,
        pool_count: u64,
    ) -> Vec<u64> {
        let mut migrated_pools = Vec::new(&env);
        
        for pool_id in 0..pool_count {
            // Read pool from old contract
            let old_pool: OldPool = env.invoke_contract(
                &old_contract,
                &Symbol::new(&env, "get_pool"),
                (pool_id,).into_val(&env),
            );
            
            // Create pool in new contract with default category
            let new_pool_id: u64 = env.invoke_contract(
                &new_contract,
                &Symbol::new(&env, "create_pool"),
                (
                    Symbol::new(&env, "Other"),  // Default category
                    old_pool.end_time,
                    old_pool.token,
                    old_pool.description,
                    old_pool.metadata_url,
                ).into_val(&env),
            );
            
            migrated_pools.push_back(new_pool_id);
        }
        
        migrated_pools
    }
    
    /// Migrate predictions for a specific pool
    pub fn migrate_predictions(
        env: Env,
        old_contract: Address,
        new_contract: Address,
        pool_id: u64,
        users: Vec<Address>,
    ) {
        for user in users.iter() {
            // Read prediction from old contract
            let prediction: Prediction = env.invoke_contract(
                &old_contract,
                &Symbol::new(&env, "get_prediction"),
                (user.clone(), pool_id).into_val(&env),
            );
            
            // Recreate prediction in new contract
            env.invoke_contract::<()>(
                &new_contract,
                &Symbol::new(&env, "place_prediction"),
                (user, pool_id, prediction.amount, prediction.outcome).into_val(&env),
            );
        }
    }
}

#[contracttype]
struct OldPool {
    pub end_time: u64,
    pub token: Address,
    pub description: String,
    pub metadata_url: String,
    // ... other fields
}

#[contracttype]
struct Prediction {
    pub amount: i128,
    pub outcome: u32,
}
```

## Step-by-Step Migration Process

### Phase 1: Preparation

1. **Audit Current State**
   ```bash
   # Count active pools
   soroban contract invoke \
     --id <CONTRACT_ID> \
     --fn get_pool_count
   
   # Export pool data
   ./scripts/export_pools.sh > pools_backup.json
   ```

2. **Deploy New Contract**
   ```bash
   # Build new contract
   cargo build --release --target wasm32-unknown-unknown
   
   # Deploy to testnet first
   soroban contract deploy \
     --wasm target/wasm32-unknown-unknown/release/predifi_contract.wasm \
     --network testnet
   ```

3. **Test Migration Script**
   ```bash
   # Deploy migration contract
   soroban contract deploy \
     --wasm target/wasm32-unknown-unknown/release/migration_contract.wasm \
     --network testnet
   
   # Run migration on testnet
   soroban contract invoke \
     --id <MIGRATION_CONTRACT_ID> \
     --fn migrate_pools \
     -- \
     --old_contract <OLD_CONTRACT_ID> \
     --new_contract <NEW_CONTRACT_ID> \
     --pool_count 100
   ```

### Phase 2: Migration Execution

1. **Pause Old Contract**
   ```bash
   soroban contract invoke \
     --id <OLD_CONTRACT_ID> \
     --fn pause \
     -- \
     --admin <ADMIN_ADDRESS>
   ```

2. **Run Migration**
   ```bash
   # Migrate pools in batches
   for i in {0..10}; do
     start=$((i * 100))
     soroban contract invoke \
       --id <MIGRATION_CONTRACT_ID> \
       --fn migrate_pools \
       -- \
       --old_contract <OLD_CONTRACT_ID> \
       --new_contract <NEW_CONTRACT_ID> \
       --start_pool $start \
       --count 100
   done
   ```

3. **Verify Migration**
   ```bash
   # Compare pool counts
   old_count=$(soroban contract invoke --id <OLD_CONTRACT_ID> --fn get_pool_count)
   new_count=$(soroban contract invoke --id <NEW_CONTRACT_ID> --fn get_pool_count)
   
   if [ "$old_count" -eq "$new_count" ]; then
     echo "✅ Migration successful"
   else
     echo "❌ Migration failed: count mismatch"
   fi
   ```

### Phase 3: Client Updates

1. **Update Frontend**
   ```typescript
   // old-api.ts
   const createPool = async (params: {
     end_time: number;
     token: string;
     description: string;
     metadata_url: string;
   }) => {
     return await oldContract.create_pool(params);
   };
   
   // new-api.ts
   import { CATEGORY_SPORTS } from './contract-bindings';
   
   const createPool = async (params: {
     category: Symbol;  // NEW
     end_time: number;
     token: string;
     description: string;
     metadata_url: string;
   }) => {
     return await newContract.create_pool(params);
   };
   ```

2. **Update Backend**
   ```python
   # old_service.py
   def create_pool(end_time, token, description, metadata_url):
       return contract.create_pool(
           end_time=end_time,
           token=token,
           description=description,
           metadata_url=metadata_url
       )
   
   # new_service.py
   from stellar_sdk import Symbol
   
   def create_pool(category, end_time, token, description, metadata_url):
       return contract.create_pool(
           category=Symbol(category),  # NEW
           end_time=end_time,
           token=token,
           description=description,
           metadata_url=metadata_url
       )
   ```

3. **Update Documentation**
   - API documentation
   - Integration guides
   - Example code
   - Error handling

### Phase 4: Verification

1. **Functional Testing**
   ```bash
   # Test pool creation
   soroban contract invoke \
     --id <NEW_CONTRACT_ID> \
     --fn create_pool \
     -- \
     --category Sports \
     --end_time 1234567890 \
     --token <TOKEN_ADDRESS> \
     --description "Test Pool" \
     --metadata_url "ipfs://test"
   
   # Test category query
   soroban contract invoke \
     --id <NEW_CONTRACT_ID> \
     --fn get_pools_by_category \
     -- \
     --category Sports
   ```

2. **Performance Testing**
   ```bash
   # Benchmark category queries
   time soroban contract invoke \
     --id <NEW_CONTRACT_ID> \
     --fn get_pools_by_category \
     -- \
     --category Sports
   ```

3. **Integration Testing**
   - Test frontend flows
   - Test backend APIs
   - Test mobile apps
   - Test third-party integrations

## Rollback Plan

If migration fails:

1. **Immediate Actions**
   ```bash
   # Unpause old contract
   soroban contract invoke \
     --id <OLD_CONTRACT_ID> \
     --fn unpause \
     -- \
     --admin <ADMIN_ADDRESS>
   
   # Revert client configurations
   git revert <MIGRATION_COMMIT>
   ```

2. **Investigation**
   - Review migration logs
   - Check data integrity
   - Identify failure points
   - Document issues

3. **Retry**
   - Fix identified issues
   - Test on testnet again
   - Schedule new migration window

## Category Assignment Strategy

For existing pools without categories, use this logic:

```rust
fn assign_category(pool: &OldPool) -> Symbol {
    // Parse description for keywords
    let desc_lower = pool.description.to_lowercase();
    
    if desc_lower.contains("sport") || desc_lower.contains("game") {
        return CATEGORY_SPORTS;
    }
    if desc_lower.contains("stock") || desc_lower.contains("market") {
        return CATEGORY_FINANCE;
    }
    if desc_lower.contains("crypto") || desc_lower.contains("bitcoin") {
        return CATEGORY_CRYPTO;
    }
    if desc_lower.contains("election") || desc_lower.contains("political") {
        return CATEGORY_POLITICS;
    }
    if desc_lower.contains("movie") || desc_lower.contains("music") {
        return CATEGORY_ENTERTAIN;
    }
    if desc_lower.contains("tech") || desc_lower.contains("ai") {
        return CATEGORY_TECH;
    }
    
    // Default category
    CATEGORY_OTHER
}
```

## Post-Migration Checklist

- [ ] All pools migrated successfully
- [ ] Category indices rebuilt
- [ ] Frontend updated and tested
- [ ] Backend updated and tested
- [ ] Mobile apps updated
- [ ] Documentation updated
- [ ] Old contract paused/deprecated
- [ ] New contract address published
- [ ] Monitoring configured
- [ ] Team trained on new system
- [ ] Users notified of changes
- [ ] Rollback plan documented

## Common Issues & Solutions

### Issue: Category Mismatch
**Symptom:** Pools appear in wrong category  
**Solution:** Run category reassignment script

### Issue: Missing Pools
**Symptom:** Some pools not migrated  
**Solution:** Check migration logs, re-run for missing IDs

### Issue: Duplicate Pools
**Symptom:** Same pool appears multiple times  
**Solution:** Deduplicate using pool metadata hash

### Issue: Performance Degradation
**Symptom:** Slow category queries  
**Solution:** Rebuild category indices, check storage TTL

## Support

For migration assistance:
- Review `CATEGORY_REFACTOR.md` for technical details
- Check `CATEGORY_QUICK_REFERENCE.md` for API changes
- Contact development team for custom migration needs

## Timeline Estimate

- **Small deployment** (<100 pools): 1-2 hours
- **Medium deployment** (100-1000 pools): 4-8 hours
- **Large deployment** (>1000 pools): 1-2 days

Plan for 2x the estimated time to account for testing and verification.
