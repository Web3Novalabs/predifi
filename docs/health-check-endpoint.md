# Health Check Endpoint

`GET /health` and `GET /api/v1/health` report the health of every dependency
the backend needs, component by component. Implementation:
`backend/src/health.rs`.

## Status model

Each component is `healthy`, `degraded`, or `unhealthy`. The overall `health`
is the worst component status, and only `unhealthy` returns **HTTP 503**, so
load balancers keep routing to an instance that is degraded but serving.

| Component | `healthy` | `degraded` | `unhealthy` |
|---|---|---|---|
| `database` | `SELECT 1` succeeds | ping ≥ 500 ms, or connection pool saturated (no idle connection, ≥ 90 % in use) | ping fails or times out (3 s) |
| `redis` | `PING` succeeds | ping ≥ 100 ms, or down/unconfigured with `PREDIFI_REDIS_REQUIRED=false` (reads fall back to the DB) | down/unconfigured with `PREDIFI_REDIS_REQUIRED=true` |
| `stellar_rpc` | `getHealth` reports `healthy` | response ≥ 1 s, or the node reports another status (e.g. catching up) | unreachable after `PREDIFI_RPC_HEALTH_RETRY_COUNT` attempts |
| `disk` | ≥ 10 % free | < 10 % free, or usage unknown | < 5 % free |
| `price_cache` | populated | — | empty (not yet fetched) |

All network probes run concurrently. The disk checked is the filesystem
holding the process working directory.

## Response

```json
{
  "status": "degraded",
  "health": "degraded",
  "service": "predifi-backend",
  "version": "0.1.0",
  "checked_at": "2026-10-17T12:00:00Z",
  "duration_ms": 41,
  "components": {
    "database": {
      "status": "healthy", "critical": true, "latency_ms": 2,
      "details": { "pool_size": 10, "pool_idle": 9, "pool_active": 1, "pool_utilization": 0.1 }
    },
    "redis": {
      "status": "degraded", "critical": false, "latency_ms": 0,
      "message": "Redis ping failed; serving uncached reads from the database"
    },
    "stellar_rpc": {
      "status": "healthy", "critical": true, "latency_ms": 38,
      "details": { "node_status": "healthy", "latest_ledger": 123456, "attempts": 1 }
    },
    "disk": {
      "status": "healthy", "critical": false,
      "details": { "mount_point": "/", "available_bytes": 51539607552, "total_bytes": 241591910400, "free_ratio": 0.213 }
    },
    "price_cache": { "status": "healthy", "critical": true, "details": { "assets": 3 } }
  },
  "dependencies": { "db": "ok", "rpc": "ok", "redis": "unreachable", "price_cache": "ok", "disk": "ok" },
  "errors": { "db": null, "rpc": null, "redis": "Redis ping failed; serving uncached reads from the database", "price_cache": null, "disk": null }
}
```

- `components` — per-component `status`, whether it is `critical`, probe
  `latency_ms`, a `message` when not healthy, and component `details`.
- `status`, `dependencies`, `errors` — kept for existing monitors.
  `status` is `ok` / `degraded` / `error`; `dependencies.*` uses the
  previous vocabulary (`ok`, `unreachable`, `not_configured`, `not_ready`,
  plus `low` / `critical` / `unknown` for disk).
- `/api/v1/health` reports `"version": "v1"`; `/health` reports the crate
  version.

`GET /ready` (readiness) and `GET /live` (liveness) are unchanged.

## Example

```bash
curl -s http://localhost:3000/health | jq '.health, (.components | map_values(.status))'
```
