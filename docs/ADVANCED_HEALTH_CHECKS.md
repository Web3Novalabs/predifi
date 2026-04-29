# Advanced Health Checks (DB/RPC/Redis)

## Overview

The predifi-backend implements deep health monitoring through two health check endpoints that verify connectivity to critical dependencies before returning a success status. This ensures load balancers and orchestrators can reliably detect when the service is degraded.

## Acceptance Criteria

✅ **Returns 503 if any dependency is unreachable**

- Database connectivity verified before returning 200
- RPC connectivity verified before returning 200
- Returns HTTP 503 (Service Unavailable) if any dependency fails

✅ **Returns 200 with detailed dependency status when healthy**

- Includes individual status for each dependency
- Clearly indicates which dependencies are operational

## Endpoints

### `GET /health`

Root-level health check endpoint for load balancer probes.

**Response (200 OK when healthy):**

```json
{
  "status": "ok",
  "service": "predifi-backend",
  "version": "0.1.0",
  "dependencies": {
    "db": "ok",
    "rpc": "ok"
  }
}
```

**Response (503 Service Unavailable when degraded):**

```json
{
  "status": "error",
  "service": "predifi-backend",
  "version": "0.1.0",
  "dependencies": {
    "db": "unreachable",
    "rpc": "ok"
  }
}
```

### `GET /api/v1/health`

Versioned health check endpoint for API consumers.

**Response (200 OK when healthy):**

```json
{
  "status": "ok",
  "version": "v1",
  "dependencies": {
    "db": "ok",
    "rpc": "ok"
  }
}
```

**Response (503 Service Unavailable when degraded):**

```json
{
  "status": "error",
  "version": "v1",
  "dependencies": {
    "db": "unreachable",
    "rpc": "ok"
  }
}
```

## Dependency Status Values

### Database (`db`)

- **`"ok"`** — PostgreSQL connection pool is operational; `SELECT 1` query succeeded
- **`"unreachable"`** — Database query failed (connection timeout, authentication error, or network issue)
- **`"not_configured"`** — Database pool was not initialized (e.g., in test environments without a database)

### RPC (`rpc`)

- **`"ok"`** — Stellar RPC endpoint is operational; `getHealth` RPC call returned HTTP 2xx
- **`"unreachable"`** — RPC endpoint is unreachable (connection timeout, HTTP error, or network issue)

## Implementation Details

### Location

- **Root health endpoint:** [src/main.rs](../backend/src/main.rs) — `health()` handler
- **V1 health endpoint:** [src/routes/v1.rs](../backend/src/routes/v1.rs) — `health()` handler
- **Tests:** [src/tests.rs](../backend/src/tests.rs) — Advanced Health Check Tests section

### Dependency Checks

#### Database Check

```rust
if let Some(db) = &state.db {
    if sqlx::query("SELECT 1").execute(db).await.is_err() {
        db_status = "unreachable";
        all_healthy = false;
    }
} else {
    db_status = "not_configured";
}
```

**How it works:**

1. Attempts to execute a trivial query (`SELECT 1`) on the connection pool
2. If the query fails, marks the database as unreachable
3. If no connection pool is configured, marks it as not_configured

**Timeout:** 2 seconds (inherited from connection pool acquire timeout)

#### RPC Check

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(2))
    .build()?;

let rpc_req = client.post(&state.config.stellar_rpc_url)
    .json(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getHealth"
    }))
    .send()
    .await;

match rpc_req {
    Ok(res) if res.status().is_success() => {}
    _ => {
        rpc_status = "unreachable";
        all_healthy = false;
    }
}
```

**How it works:**

1. Creates a dedicated HTTP client with a 2-second timeout
2. Posts a `getHealth` JSON-RPC request to the configured Stellar RPC endpoint
3. Considers the RPC healthy only if the request succeeds and returns HTTP 2xx
4. Falls back to a default client if configuration fails

**Timeout:** 2 seconds

### HTTP Status Codes

- **200 OK** — All dependencies are reachable and operational
- **503 Service Unavailable** — One or more dependencies are unreachable
- **429 Too Many Requests** — Rate limit exceeded (50 request burst limit)

## Test Coverage

Located in [src/tests.rs](../backend/src/tests.rs):

| Test                                                   | Purpose                                              |
| ------------------------------------------------------ | ---------------------------------------------------- |
| `api_v1_health_returns_200_with_dependency_status`     | Verifies v1 health includes dependency status        |
| `root_health_returns_200_with_dependency_status`       | Verifies root health includes dependency status      |
| `api_v1_health_reports_db_not_configured_without_pool` | Confirms db reports as "not_configured" without pool |
| `root_health_reports_db_not_configured_without_pool`   | Confirms db reports as "not_configured" without pool |
| `api_v1_health_status_is_ok_when_healthy`              | Verifies status is "ok" when all systems are healthy |
| `root_health_status_is_ok_when_healthy`                | Verifies status is "ok" when all systems are healthy |
| `health_includes_cargo_version`                        | Verifies version field is populated from Cargo.toml  |

### Running Tests

```bash
cd backend
cargo test -- --test-threads=1
```

## Configuration

The health check uses the following environment variables:

- **`STELLAR_RPC_URL`** — Stellar RPC endpoint to check (default: `https://soroban-testnet.stellar.org`)
- **`DATABASE_URL`** — PostgreSQL connection string (no default; required for health check to verify connectivity)

## Usage Examples

### Docker / Kubernetes

```yaml
# Kubernetes liveness probe
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3

# Kubernetes readiness probe
readinessProbe:
  httpGet:
    path: /api/v1/health
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 3
  timeoutSeconds: 2
  failureThreshold: 2
```

### curl

```bash
# Check root health
curl -v http://localhost:3000/health

# Check API v1 health
curl -v http://localhost:3000/api/v1/health

# Parse JSON response
curl -s http://localhost:3000/health | jq '.dependencies'

# Exit with non-zero status if unhealthy
curl -f http://localhost:3000/health > /dev/null || echo "Service is unhealthy"
```

### Monitoring / Alerting

```bash
# Monitor every 5 seconds and alert if status degrades
watch -n 5 'curl -s http://localhost:3000/health | jq "."'

# Continuous monitoring with timestamp
while true; do
  echo "[$(date)] Health: $(curl -s http://localhost:3000/health | jq -r '.status')"
  sleep 5
done
```

## Performance Considerations

- **Connection pool timeout:** 2 seconds per dependency check
- **Total health check time:** ~2-4 seconds (sequential checks with timeouts)
- **Rate limiting:** 50 request burst, 5 per second thereafter

The health check is relatively expensive due to network calls, so it's recommended to:

- Probe infrequently (every 5-10 seconds for liveness, 3-5 for readiness)
- Use separate endpoints for different probe types
- Cache results client-side if frequent checks are needed

## Troubleshooting

### Database Reported as Unreachable

1. **Verify DATABASE_URL is set correctly:**

   ```bash
   echo $DATABASE_URL
   ```

2. **Test PostgreSQL connectivity:**

   ```bash
   psql "$DATABASE_URL" -c "SELECT 1"
   ```

3. **Check connection pool configuration:**
   - `DB_MAX_CONNECTIONS` (default: 10)
   - `DB_MIN_CONNECTIONS` (default: 1)
   - `DB_ACQUIRE_TIMEOUT_SECS` (default: 30)

### RPC Reported as Unreachable

1. **Verify STELLAR_RPC_URL is set correctly:**

   ```bash
   echo $STELLAR_RPC_URL
   ```

2. **Test RPC connectivity:**

   ```bash
   curl -X POST "$STELLAR_RPC_URL" \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
   ```

3. **Check network connectivity:**
   ```bash
   ping $(echo $STELLAR_RPC_URL | sed 's|https://||' | cut -d/ -f1)
   ```

### Always Returns 503

1. **Check logs for errors:**

   ```bash
   RUST_LOG=debug cargo run
   ```

2. **Verify all dependencies are accessible**
3. **Review dependency status in response** to identify which component is failing

## Related Documentation

- [Health Check Endpoint](./health-check-endpoint.md) — Original basic health check
- [Troubleshooting](./troubleshooting.md) — General backend troubleshooting guide
- Backend source:
  - [main.rs](../backend/src/main.rs)
  - [routes/v1.rs](../backend/src/routes/v1.rs)
  - [config.rs](../backend/src/config.rs)
