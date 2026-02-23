# Category System - Quick Reference

## Available Categories

```rust
CATEGORY_SPORTS      // Sports-related predictions
CATEGORY_FINANCE     // Financial markets
CATEGORY_CRYPTO      // Cryptocurrency predictions
CATEGORY_POLITICS    // Political events
CATEGORY_ENTERTAIN   // Entertainment industry
CATEGORY_TECH        // Technology predictions
CATEGORY_OTHER       // Miscellaneous
```

## Creating a Pool

**Old (String-based):**
```rust
client.create_pool(
    &end_time,
    &token,
    &description,
    &metadata_url,
)
```

**New (Symbol-based):**
```rust
client.create_pool(
    &CATEGORY_SPORTS,  // ← NEW: Category parameter first
    &end_time,
    &token,
    &description,
    &metadata_url,
)
```

## Querying Pools by Category

```rust
// Get all sports pools
let sports_pools = client.get_pools_by_category(&CATEGORY_SPORTS);

// Iterate through results
for pool_id in sports_pools.iter() {
    let pool = client.get_pool(&pool_id);
    // Process pool...
}
```

## Error Handling

```rust
// Invalid category panics with "Invalid category"
let bad_category = Symbol::new(&env, "BadCat");
client.create_pool(&bad_category, ...);  // ❌ PANICS

// Valid category works
client.create_pool(&CATEGORY_SPORTS, ...);  // ✅ OK
```

## Test Examples

```rust
#[test]
fn test_create_sports_pool() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = setup_client(&env);
    
    let pool_id = client.create_pool(
        &CATEGORY_SPORTS,
        &100u64,
        &token_address,
        &String::from_str(&env, "Match Winner"),
        &String::from_str(&env, "ipfs://metadata"),
    );
    
    assert_eq!(pool_id, 0);
}

#[test]
fn test_query_by_category() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = setup_client(&env);
    
    // Create multiple pools
    client.create_pool(&CATEGORY_SPORTS, ...);
    client.create_pool(&CATEGORY_FINANCE, ...);
    client.create_pool(&CATEGORY_SPORTS, ...);
    
    // Query sports pools
    let sports = client.get_pools_by_category(&CATEGORY_SPORTS);
    assert_eq!(sports.len(), 2);
}
```

## Frontend Integration

```typescript
// Import category constants
import { 
  CATEGORY_SPORTS,
  CATEGORY_FINANCE,
  CATEGORY_CRYPTO 
} from './contract-bindings';

// Create pool
const createPool = async (category: Symbol) => {
  const tx = await contract.create_pool({
    category,
    end_time: Date.now() + 86400,
    token: tokenAddress,
    description: "Pool description",
    metadata_url: "ipfs://..."
  });
  return tx;
};

// Query pools
const getSportsPools = async () => {
  const pools = await contract.get_pools_by_category({
    category: CATEGORY_SPORTS
  });
  return pools;
};
```

## Migration Checklist

- [ ] Update all `create_pool` calls to include category
- [ ] Import category constants where needed
- [ ] Update frontend pool creation forms
- [ ] Add category filter to pool listing UI
- [ ] Update API endpoints to handle categories
- [ ] Test all pool creation flows
- [ ] Update documentation and examples

## Common Mistakes

❌ **Wrong:** Using string instead of Symbol
```rust
client.create_pool("Sports", ...)  // Compile error
```

✅ **Correct:** Using Symbol constant
```rust
client.create_pool(&CATEGORY_SPORTS, ...)
```

❌ **Wrong:** Creating custom category
```rust
let custom = Symbol::new(&env, "MyCategory");
client.create_pool(&custom, ...)  // Panics: Invalid category
```

✅ **Correct:** Using predefined category
```rust
client.create_pool(&CATEGORY_OTHER, ...)
```

❌ **Wrong:** Querying without validation
```rust
let pools = client.get_pools_by_category(&bad_symbol);  // Panics
```

✅ **Correct:** Using valid category
```rust
let pools = client.get_pools_by_category(&CATEGORY_CRYPTO);
```

## Performance Tips

1. **Cache category queries:** Results don't change frequently
2. **Batch operations:** Group pools by category for efficient processing
3. **Use appropriate category:** Helps users find relevant pools faster
4. **Index optimization:** Category indices are optimized for fast lookups

## Support

For questions or issues:
- Check `CATEGORY_REFACTOR.md` for detailed documentation
- Review test cases in `src/test.rs`
- See integration tests in `src/integration_test.rs`
