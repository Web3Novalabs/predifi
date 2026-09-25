# JWT Security Hardening Implementation

**Issue:** #1554 Security: Backend JWT token security hardening

## Overview

This document details the security hardening measures implemented for JWT token handling in the PrediFi backend.

## Security Improvements

### 1. Token Type Enforcement

**Problem:** Refresh tokens could be misused as access tokens, and vice versa.

**Solution:** Added `token_type` claim to all JWT tokens.
- Access tokens: `token_type: "access"` (1-hour expiry)
- Refresh tokens: `token_type: "refresh"` (7-day expiry)

**Implementation:**
```rust
pub fn verify_jwt_token_strict(
    token: &str,
    secret: &str,
    expected_type: &str,
    current_key_version: u64,
) -> Result<PredifiClaims, JwtVerifyError>
```

This ensures:
- Refresh endpoints reject access tokens
- Access endpoints reject refresh tokens
- Token type is cryptographically bound to the token signature

### 2. Secret Key Rotation Support

**Problem:** No mechanism to rotate signing secrets without invalidating all existing tokens.

**Solution:** Added `key_version` claim to all tokens.

**Features:**
- Each token includes the key version under which it was issued
- Server tracks `current_key_version` in configuration
- During verification:
  - Tokens from future versions are rejected (indicates client clock/sync issues)
  - Tokens from old versions are logged and accepted (allows gradual rollout)
  - This enables gradual key rotation without service disruption

**Rotation Strategy:**
1. Deploy new `current_key_version` in server config
2. Old tokens remain valid (issued under old version)
3. New tokens issued under new version
4. After old token expiry window (7 days for refresh, 1 hour for access), all tokens are from new version

### 3. Token Revocation Support

**Problem:** No mechanism to invalidate tokens before expiry (logout, password change, etc.).

**Solution:** Implemented Redis-backed token revocation lists in `jwt_security.rs`.

**Functions:**
```rust
// Revoke a specific token by JTI (JWT ID)
pub async fn revoke_token(
    cache: &RedisCache,
    token_jti: &str,
    ttl_secs: u64,
) -> Result<(), String>

// Revoke all tokens for a user (on logout, password change)
pub async fn revoke_user_tokens(cache: &RedisCache, user_address: &str) -> Result<(), String>

// Check if a token is still valid
pub async fn is_token_valid_for_user(
    cache: &RedisCache,
    user_address: &str,
    issued_at: u64,
) -> bool
```

**Usage:**
- On logout: Call `revoke_user_tokens(cache, user_address)`
- On password change: Call `revoke_user_tokens(cache, user_address)`
- On suspicious activity: Call `revoke_token(cache, jti, ttl_secs)`

**Implementation Details:**
- Revocation entries stored in Redis with TTL matching token expiry
- Revoke-before timestamps allow efficient bulk invalidation
- Redis unavailability defaults to accepting tokens (fail-open strategy)
  - *Note:* Consider fail-closed strategy for critical environments

### 4. Refresh Token Rotation

**Problem:** Long-lived refresh tokens increase compromise window.

**Solution:** Implemented refresh token lifecycle with automatic rotation.

**Token Lifetimes:**
- Access tokens: 1 hour (existing)
- Refresh tokens: 7 days (new)

**Refresh Endpoint Flow:**
```
POST /api/v1/auth/refresh { "refresh_token": "..." }
  → Verify token (strict: type="refresh", matching key_version)
  → Fail closed if Redis is unavailable
  → Reject if JTI is already revoked (replay)
  → Revoke presented JTI
  → Issue new access token + new refresh token (rotated)
  → Return both tokens
```

Rate-limited at the Token tier: **10 requests / 60 seconds per IP**.

**Benefits:**
- Reduces impact of refresh token compromise
- Allows tracking token usage patterns (rotation audits)
- Supports forced logout by revoking refresh tokens

### 5. Algorithm Confusion Prevention

**Current Protection:**
- Hard-coded `Algorithm::HS256` in all verification paths
- NO algorithm negotiation from token header
- Prevents:
  - `alg: none` attacks (unsigned tokens)
  - RS256 ↔ HS256 confusion attacks
  - Unknown algorithm attacks

**Code Location:** `predifi/backend/src/jwt.rs`, line ~175
```rust
let validation = Validation::new(Algorithm::HS256);
```

**Why This Works:**
- The `jsonwebtoken` crate enforces algorithm matching
- Token must have `alg: HS256` in header
- Signature verified with HMAC-SHA256
- Any algorithm mismatch is cryptographically detected

### 6. Rate Limiting for Token Operations

**Problem:** No specific rate limiting for token endpoints.

**Solution:** Added `Token` rate limit tier.

**Configuration:**
- 10 requests per 60 seconds per IP address
- Applied to `/auth/refresh` and similar token endpoints
- Stricter than other endpoints to prevent brute-force attacks

**Usage:**
```rust
use crate::rate_limit::{RateLimitTier, with_rate_limit};

let router = with_rate_limit(token_router, RateLimitTier::Token);
```

### 7. Enhanced Secret Key Management

**Startup Validation:**
- Minimum 32-byte secret enforced
- Production rejects default dev secret
- Error messages guide operators to use strong keys

**Best Practices (Operational):**
- Store secrets in environment variables (or better: secrets manager like Vault, K8s Secrets)
- Rotate secrets following the key_version strategy
- Never commit secrets to version control
- Monitor secret access in logs
- Use separate secrets per environment (dev, staging, prod)

**Production Checklist:**
- [ ] Use cryptographically random 32+ byte secret
- [ ] Store in secrets management system (not git, not logs)
- [ ] Rotate secrets periodically (e.g., quarterly)
- [ ] Implement secret rotation without service restart
- [ ] Monitor failed token validations (possible key mismatch)

## Usage Examples

### Signing a Token (with key version)
```rust
use crate::jwt::sign_jwt_with_type;

// Access token
let token = sign_jwt_with_type(
    "GABC123",           // user address
    secret,              // signing secret
    now_unix,            // current Unix timestamp
    "access",            // token type
    0                    // current key version
)?;

// Refresh token (7-day expiry)
let refresh = sign_jwt_with_type(
    "GABC123",
    secret,
    now_unix,
    "refresh",
    0
)?;
```

### Verifying a Token (strict mode)
```rust
use crate::jwt::verify_jwt_token_strict;

// Verify access token
let claims = verify_jwt_token_strict(
    &token,
    secret,
    "access",           // expected type
    current_key_version // server's current key version
)?;

// Use claims.sub as wallet address
```

### Revoking Tokens
```rust
use crate::jwt_security::{revoke_token, revoke_user_tokens};

// Logout: revoke all user tokens
revoke_user_tokens(&redis_cache, user_address).await?;

// Suspicious activity: revoke specific token
let jti = generate_jti(user_address, issued_at, 0);
revoke_token(&redis_cache, &jti, ttl_secs).await?;
```

## Security Checklist

- [x] Token expiration enforcement (1 hour for access, 7 days for refresh)
- [x] Refresh token rotation mechanism
- [x] Secret key minimum length (32 bytes)
- [x] Algorithm confusion prevention (hard-coded HS256)
- [x] Algorithm enforcement documented
- [x] Production secret validation
- [x] Token type binding (prevent refresh→access misuse)
- [x] Key version tracking (support secret rotation)
- [x] Token revocation support (Redis-backed)
- [x] Rate limiting for token endpoints (10 req/60s)
- [x] Session fixation prevention (existing in session.rs)
- [x] Idle timeout enforcement (existing in session.rs)

## Remaining Considerations

### WebSocket Token Security
- Current: Accepts tokens as query parameters (logged in access logs)
- Recommendation: Enforce Authorization header only in production
- Consider: Separate short-lived WebSocket-specific tokens

### Token Compression
- Potential future: Reduce payload size (more network friendly)
- Trade-off: More difficult to inspect/audit

### Distributed Key Rotation
- Current: Single secret per environment
- Future: Multi-datacenter key rotation without coordination

### Audit Logging
- Recommendation: Log all token validations (successes + failures)
- Use: Detect compromised tokens, patterns of abuse

## Testing

All security functions include unit tests:
- `jwt.rs`: Format validation, signature verification, expiration
- `jwt_security.rs`: JTI generation determinism, key version tracking
- `constants.rs`: Rate limiting tiers

Run tests:
```bash
cargo test --lib jwt
cargo test --lib jwt_security
cargo test --lib rate_limit
```

## Related Issues
- #1554: JWT token security hardening (this implementation)
- Session fixation prevention: Already implemented in session.rs
- Rate limiting: Token tier applied to `/auth/refresh` and `/ws`

## References
- [OWASP JWT Best Practices](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
- [RFC 7519: JSON Web Token (JWT)](https://tools.ietf.org/html/rfc7519)
- [JWT Algorithm Confusion Attacks](https://auth0.com/blog/critical-vulnerabilities-in-json-web-token-libraries/)
