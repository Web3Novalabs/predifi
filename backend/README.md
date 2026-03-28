# predifi-backend

A minimal Axum HTTP server for the PrediFi platform, featuring a custom Tower
request-logging middleware and a versioned API router layout.

Every request is logged to stdout with its method, path, response status, and duration:

```text
[REQ] GET /health -> 200 OK (1ms)
[REQ] GET /api/v1/health -> 200 OK (0ms)
[REQ] GET /missing -> 404 Not Found (0ms)
```

---

## Prerequisites

| Tool | Version | Install |
| :--- | :--- | :--- |
| Rust + Cargo | stable | `curl https://sh.rustup.rs -sSf \| sh` |
| (optional) cargo-watch | any | `cargo install cargo-watch` |

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
cargo run
```

The server listens on `http://localhost:3000`.

Verify it is running:

```bash
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

## Run tests

Tests use Tower's `.oneshot()` helper, so no live server is needed.

```bash
cargo test
```

---

## Project layout

```text
src/
|-- main.rs            # top-level app router and server entry point
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

| Type | Role |
| :--- | :--- |
| `LoggingLayer` | Factory - wraps any service with `LoggingService` |
| `LoggingService<S>` | Intercepts each request/response pair and emits a log line |

Attach it to a router with:

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(LoggingLayer);
```
