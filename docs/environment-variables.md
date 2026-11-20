# Environment Variables

Complete reference for every variable read by `backend/src/config.rs`.
Copy `backend/.env.example` to `backend/.env` and fill in placeholders.

| Variable | Type | Default | Required | Affects |
|----------|------|---------|:--------:|---------|
| `DATABASE_URL` | string | — | Yes | PostgreSQL connection pool |
| `REDIS_URL` | string | — | Yes | Rate limiting and caches |
| `PORT` | u16 | `3000` | No | Backend HTTP listen port |
| `PREDIFI_SECRET_KEY` | string | dev placeholder | No | JWT signing (32+ bytes) |
| `PREDIFI_JWT_KEY_VERSION` | u32 | `0` | No | Bump to invalidate tokens |
| `PREDIFI_ADMIN_API_KEY` | string | unset | For admin endpoint | Shared secret for `PATCH /api/v1/admin/log-level`; loaded once at startup and compared in constant time. Must be at least 32 bytes; without it all admin requests get 401. |
| `PREDIFI_TRUST_PROXY_HEADERS` | bool | `false` | No | Trust `X-Forwarded-For`/`X-Real-IP`/`Forwarded` for rate-limit keys. Enable only behind a proxy that overwrites these headers. |
| `PREDIFI_INDEXER_MAX_BATCH_SIZE` | u32 | `500` | No | Stellar event batch size |
| `PREDIFI_INDEXER_API_KEY` | string | unset | Only in production | Shared secret required via `x-indexer-key` (or `Authorization: Bearer`) on `/api/v1/indexer/*` ingest routes and accepted for payout endpoints |
| `PREDIFI_SENTRY_DSN` | string | unset | No | Error reporting |
