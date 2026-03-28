# predifi-backend

A minimal Axum HTTP server with a custom request logging middleware.

Every request is logged to stdout with its HTTP method, path, response status, and duration.

```
[REQ] GET /health → 200 OK (1ms)
[REQ] GET /missing → 404 Not Found (0ms)
```

---

## Project layout

```
src/
├── lib.rs      — router setup and server entry point
├── request_logger.rs   — LoggingLayer / LoggingService middleware
└── tests.rs     — unit and integration tests
```

---

## Run

```bash
cargo run
```

The server listens on `http://localhost:3000`.

Try it:

```bash
curl http://localhost:3000/
curl http://localhost:3000/health
curl http://localhost:3000/missing    # produces a 404
```

---

## Test

```bash
cargo test
```

Tests run without a live server — Tower's `.oneshot()` helper fires requests
directly into the service in memory.

---

## How the middleware works (plain English)

A Tower middleware sits between the server and your route handlers. Every
request passes *through* it before reaching a handler, and every response
passes back *through* it on the way out.

```
HTTP request
     │
     ▼
LoggingLayer      ← records method + path, starts a timer
     │
     ▼
Route handler     ← your normal code runs here
     │
     ▼
LoggingLayer      ← records status + elapsed time, prints the log line
     │
     ▼
HTTP response
```

The implementation has two parts:

| Type | Role |
|---|---|
| `LoggingLayer` | Factory — wraps any service with `LoggingService` |
| `LoggingService<S>` | Does the actual work per request |

`LoggingLayer` implements Tower's `Layer` trait.
`LoggingService` implements Tower's `Service` trait.

Attach it to an Axum router with:

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(LoggingLayer);
```