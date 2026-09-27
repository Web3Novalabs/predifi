# Runtime Log Level Configuration

The backend exposes an authenticated administrative endpoint that allows
changing the active log level without restarting the process.

## Endpoint

```
PATCH /api/v1/admin/log-level
```

### Request

```json
{
  "level": "debug"
}
```

| Field | Description |
| :--- | :--- |
| `level` | New minimum log level.  Accepted values: `error`, `warn`, `info`, `debug`, `trace` |

### Response — success (200)

```json
{
  "status": "ok",
  "level": "debug"
}
```

### Response — unauthorised (401)

```json
{
  "error": "invalid or missing admin credentials"
}
```

### Response — bad request (400)

```json
{
  "error": "invalid log level: verbose"
}
```

## Authentication

The endpoint requires an `x-admin-api-key` header whose value matches the
`PREDIFI_ADMIN_API_KEY` environment variable.

```bash
curl -X PATCH http://localhost:3000/api/v1/admin/log-level \
  -H "Content-Type: application/json" \
  -H "x-admin-api-key: $PREDIFI_ADMIN_API_KEY" \
  -d '{"level":"debug"}'
```

## Behaviour

- The new level takes effect immediately for all subsequent log events.
- Spans already in flight keep the level that was active when they were created.
- The level reverts to the configured default (`RUST_LOG` / `LOG_LEVEL`) on
  process restart because the subscriber is rebuilt from the config value at
  startup.
- Changes are written to the log at `INFO` level for audit purposes:

```json
{
  "timestamp": "...",
  "level": "INFO",
  "message": "log level updated via admin endpoint",
  "log_level": "debug"
}
```

## Environment variables

| Variable | Required | Description |
| :--- | :--- | :--- |
| `PREDIFI_ADMIN_API_KEY` | Yes | Shared secret for the admin endpoint.  Must be set in production. |
| `RUST_LOG` | No | Default log level at startup (default: `info`). |
