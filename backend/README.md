# predifi-backend

A minimal Axum HTTP server for the PrediFi platform, featuring a custom Tower request-logging middleware.

Every request is logged to stdout with its method, path, response status, and duration:

```
[REQ] GET /health → 200 OK (1ms)
[REQ] GET /missing → 404 Not Found (0ms)
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
curl http://localhost:3000/           # 200 — welcome message
curl http://localhost:3000/health     # 200 — {"status":"ok","service":"predifi-backend","version":"0.1.0"}
curl http://localhost:3000/missing    # 404 — unknown route
```

Auto-restart on file changes (requires `cargo-watch`):

```bash
cargo watch -x run
```

---

## Run tests

Tests use Tower's `.oneshot()` helper — no live server needed.

```bash
cargo test
```

---

## Project layout

```
src/
├── main.rs            — router setup and server entry point
├── request_logger.rs  — LoggingLayer / LoggingService middleware
└── tests.rs           — unit and integration tests
```

---

## How the middleware works

A Tower middleware wraps every route handler. Requests pass through it on the way in and responses pass through it on the way out — giving a single place to observe both.

```
HTTP request
     │
     ▼
LoggingLayer      ← records method + path, starts a timer
     │
     ▼
Route handler     ← normal handler code runs here
     │
     ▼
LoggingLayer      ← records status + elapsed time, prints the log line
     │
     ▼
HTTP response
```

| Type | Role |
| :--- | :--- |
| `LoggingLayer` | Factory — wraps any service with `LoggingService` |
| `LoggingService<S>` | Intercepts each request/response pair and emits a log line |

Attach it to a router with:

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(LoggingLayer);
```
