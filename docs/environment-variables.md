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
| `PREDIFI_INDEXER_MAX_BATCH_SIZE` | u32 | `500` | No | Stellar event batch size |
| `PREDIFI_SENTRY_DSN` | string | unset | No | Error reporting |
