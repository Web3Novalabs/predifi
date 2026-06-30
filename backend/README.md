# predifi-backend

A minimal Axum HTTP server for the PrediFi platform, featuring a custom Tower
request-logging middleware, environment-based configuration, and a PostgreSQL
connection pool scaffold.

Every request is logged through `tracing` with method, path, response status,
and duration fields:

```text
2026-03-29T12:00:00Z  INFO request complete method=GET path=/health status=200 OK elapsed_ms=1
2026-03-29T12:00:01Z  INFO request complete method=GET path=/api/v1/health status=200 OK elapsed_ms=0
2026-03-29T12:00:02Z  INFO request complete method=GET path=/missing status=404 Not Found elapsed_ms=0
```

---

## Prerequisites

| Tool                   | Version | Install                                |
| :--------------------- | :------ | :------------------------------------- |
| Rust + Cargo           | stable  | `curl https://sh.rustup.rs -sSf \| sh` |
| (optional) cargo-watch | any     | `cargo install cargo-watch`            |

Verify your installation:

```bash
rustc --version
cargo --version
```

---

## Installation

```bash
# from the repo root
cd backend
cargo build
```

---

## Run the dev server

```bash
cp .env.example .env
cargo run
```

The server listens on `http://localhost:3000`.

Verify it is running:

```bash
curl http://localhost:3000/           # 200 — welcome message
curl http://localhost:3000/health     # 200 — {"status":"ok","service":"predifi-backend","version":"0.1.0"}
curl http://localhost:3000/missing    # 404 — unknown route
curl http://localhost:3000/               # 200 - welcome message
curl http://localhost:3000/health         # 200 - {"status":"ok","version":"v1"}
curl http://localhost:3000/api/v1         # 200 - version discovery
curl http://localhost:3000/api/v1/health  # 200 - {"status":"ok","version":"v1"}
curl http://localhost:3000/missing        # 404 - unknown route
```

Auto-restart on file changes (requires `cargo-watch`):

```bash
cargo watch -x run
```

---

## Environment configuration

The backend loads `.env` automatically at startup (via `dotenvy`) and then
reads environment variables into a typed `Config` struct.

| Variable                  | Default                                               | Description                    |
| :------------------------ | :---------------------------------------------------- | :----------------------------- |
| `APP_HOST`                | `0.0.0.0`                                             | Host interface to bind         |
| `APP_PORT`                | `3000`                                                | HTTP port                      |
| `RUST_LOG`                | `info`                                                | Tracing filter level           |
| `DATABASE_URL`            | `postgres://postgres:postgres@localhost:5432/predifi` | PostgreSQL DSN                 |
| `DB_MAX_CONNECTIONS`      | `10`                                                  | SQLx pool max connections      |
| `DB_MIN_CONNECTIONS`      | `1`                                                   | SQLx pool min connections      |
| `DB_ACQUIRE_TIMEOUT_SECS` | `30`                                                  | Pool acquire timeout (seconds) |

If an environment variable has an invalid value (for example, a non-numeric
port), startup fails with a clear configuration error.

---

## SQLx connection pool

The application initializes a PostgreSQL pool at startup using
`sqlx::postgres::PgPoolOptions` with sensible defaults from `Config`.

The pool uses lazy mode (`connect_lazy`) to keep local development simple while
still validating pool configuration and creating a reusable `PgPool` handle.

---

## Structured tracing

`tracing-subscriber` is initialized once in `main`, using `RUST_LOG` for
filtering. The request middleware now emits structured tracing events rather
than plain `println!` output.

---

## Docker (multi-stage)

Build and run the backend container from this directory:

```bash
docker build -t predifi-backend:local .
docker run --rm -p 3000:3000 --env-file .env predifi-backend:local
```

The Dockerfile uses a multi-stage build to compile a release binary in a Rust
builder image, then copies only the binary into a slim runtime image.

---

## Run tests

Tests use Tower's `.oneshot()` helper, so no live server is needed.

```bash
cargo test
```

---

## Seed local database

A `predifi-seed` binary populates the local PostgreSQL database with
deterministic, idempotent sample data — pools across every state/category,
predictions from fixture wallets, and referral payments — so the API can be
exercised end-to-end without waiting for on-chain events to be indexed.

```bash
# insert seed data (idempotent — safe to re-run)
cargo run --bin predifi-seed

# truncate first, then seed fresh
cargo run --bin predifi-seed -- --fresh

# generate more pools
cargo run --bin predifi-seed -- --num-pools 25

cargo run --bin predifi-seed -- --help
```

The seeder runs all migrations before inserting, so it is safe to invoke
against a freshly created database. All inserts use `ON CONFLICT DO NOTHING` /
`DO UPDATE` keyed on natural primary keys, so re-running produces the same
final state. The fixture data is generated in `src/seed.rs`; the binary
entry point lives in `src/bin/seed.rs`.

---

## Project layout

```text
src/
|-- main.rs            # server binary entry point
|-- lib.rs             # library crate shared with the seed binary
|-- bin/
|   `-- seed.rs        # `predifi-seed` binary (local DB seeding)
|-- seed.rs            # seed data fixtures + idempotent inserts
|-- config.rs          # typed env configuration loader
|-- db.rs              # SQLx PostgreSQL pool initialization
|-- request_logger.rs  # LoggingLayer / LoggingService middleware
|-- routes/
|   |-- mod.rs         # API router tree (/api)
|   `-- v1.rs          # version 1 routes (/api/v1)
`-- tests.rs           # router tests
```

---

## API router structure

The backend now uses a nested Axum router layout for future scalability:

```rust
Router::new()
    .route("/", get(root))
    .route("/health", get(v1::health))
    .nest("/api", routes::router())
```

Inside `routes::router()`, versioned routes are nested under `/v1`, which gives
the service a clear expansion path for future versions such as `/api/v2`.

---

## How the middleware works

A Tower middleware wraps every route handler. Requests pass through it on the
way in and responses pass through it on the way out, giving a single place to
observe both.

```text
HTTP request
     |
     v
LoggingLayer      <- records method + path, starts a timer
     |
     v
Route handler     <- normal handler code runs here
     |
     v
LoggingLayer      <- records status + elapsed time, prints the log line
     |
     v
HTTP response
```

| Type                | Role                                                       |
| :------------------ | :--------------------------------------------------------- |
| `LoggingLayer`      | Factory - wraps any service with `LoggingService`          |
| `LoggingService<S>` | Intercepts each request/response pair and emits a log line |

Attach it to a router with:

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(LoggingLayer);
```

---

## `GET /api/v1/markets/:id/predictions` — cursor-paginated predictions per market

Returns the predictions placed in a specific prediction market (pool), ordered
newest first, with cursor-based pagination.

### Query parameters

| Parameter | Type    | Required | Description                                                       |
| :-------- | :------ | :------- | :---------------------------------------------------------------- |
| `after`   | integer | No       | Cursor value from the previous page's `next_cursor` field         |
| `limit`   | integer | No       | Page size, 1–100 (default 20)                                     |

### Example — first page

```bash
curl "http://localhost:3000/api/v1/markets/42/predictions?limit=3"
```

```json
{
  "status": "success",
  "data": {
    "market_id": 42,
    "predictions": [
      { "id": 305, "pool_id": 42, "user_address": "GABC...", "outcome": 1, "amount": 500, "created_at": "2026-06-29T08:00:00Z" },
      { "id": 304, "pool_id": 42, "user_address": "GDEF...", "outcome": 0, "amount": 200, "created_at": "2026-06-28T22:30:00Z" },
      { "id": 302, "pool_id": 42, "user_address": "GXYZ...", "outcome": 1, "amount": 100, "created_at": "2026-06-28T10:15:00Z" }
    ],
    "total": 47,
    "limit": 3,
    "next_cursor": 302
  }
}
```

### Example — next page

```bash
curl "http://localhost:3000/api/v1/markets/42/predictions?limit=3&after=302"
```

Pass `next_cursor` from the previous response as `after`. When `next_cursor` is
`null` you have reached the last page.

### Error responses

| Status | Code                   | When                             |
| :----- | :--------------------- | :------------------------------- |
| 404    | `NOT_FOUND`            | The market ID does not exist     |
| 503    | `DATABASE_UNAVAILABLE` | No database pool configured      |
| 500    | `INTERNAL_ERROR`       | Unexpected database query error  |
