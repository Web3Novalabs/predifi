# PrediFi API Reference

**Version**: 1.0.0  
**OpenAPI Specification**: 3.1.0 / 3.0.0 Compatible  
**Base URL**: `https://api.predifi.com/v1` (Production) / `http://localhost:8080/api/v1` (Local Dev)  
**OpenAPI Spec Endpoint**: `GET /api-docs/openapi.json`  

---

## Table of Contents

1. [Overview & Specification Management](#overview--specification-management)
2. [Authentication Requirements](#authentication-requirements)
3. [Rate Limiting Details](#rate-limiting-details)
4. [WebSocket Subscription Documentation](#websocket-subscription-documentation)
5. [API Endpoints Reference](#api-endpoints-reference)
   - [Health & System Diagnostics](#health--system-diagnostics)
   - [Prediction Market Pools](#prediction-market-pools)
   - [Protocol Statistics & Fees](#protocol-statistics--fees)
   - [Leaderboards & User Rankings](#leaderboards--user-rankings)
   - [Predictions & User History](#predictions--user-history)
   - [Referrals & Rewards](#referrals--rewards)
   - [Indexer Ingestion (Internal)](#indexer-ingestion-internal)
   - [Tags, Notifications & Templates](#tags-notifications--templates)
6. [Request and Response Examples](#request-and-response-examples)
7. [Error Handling & Error Envelopes](#error-handling--error-envelopes)
8. [Client SDK Generation](#client-sdk-generation)

---

## Overview & Specification Management

The PrediFi backend API is an asynchronous Rust service built on Axum and Tokio. It provides indexed reads, real-time WebSocket feeds, and cached calculations for the PrediFi prediction market protocol running on Stellar/Soroban.

### Generating and Checking the OpenAPI Spec

The OpenAPI definition is maintained in code via `backend/src/openapi.rs` using `utoipa`. The compiled CLI tool `predifi-openapi` enables automated export, validation, and continuous integration verification without needing to spin up database dependencies:

```bash
# Print OpenAPI JSON spec to stdout
cargo run --bin predifi-openapi

# Export OpenAPI JSON spec to file
cargo run --bin predifi-openapi -- --out openapi.json

# Check if the spec file is up-to-date (CI validation)
cargo run --bin predifi-openapi -- --check openapi.json
```

The live server also exposes the schema directly:
- **Raw JSON Schema**: `GET /api-docs/openapi.json`
- **Interactive Swagger UI**: Served at `/swagger-ui/` in environments where `utoipa-swagger-ui` is enabled.

---

## Authentication Requirements

PrediFi utilizes JSON Web Tokens (JWT) signed using HMAC-SHA256 (`HS256`) for authenticating protected API routes and WebSocket connections.

### Token Header Format

Clients must pass the token in the standard HTTP `Authorization` header:

```http
Authorization: Bearer <jwt_access_token>
```

### JWT Claims Payload

```json
{
  "sub": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
  "type": "access",
  "exp": 1774000000,
  "iat": 1773996400,
  "key_version": 0
}
```

| Claim | Type | Description |
|-------|------|-------------|
| `sub` | string | Stellar public address (G... format) of the authenticated user. |
| `type` | string | Token type: `"access"` for request authorization, `"refresh"` for token rotation. |
| `exp` | integer | Expiration UNIX timestamp in seconds. |
| `iat` | integer | Issued-at UNIX timestamp in seconds. |
| `key_version` | integer | Active key version for rapid server-side key rotation. |

### Token Expiration and Lifecycles

- **Access Token TTL**: 1 hour (3,600 seconds).
- **Refresh Token TTL**: 7 days (604,800 seconds).

### Obtaining and Refreshing Tokens

1. **Sign-In Flow**:
   - The user connects their wallet (e.g., Freighter).
   - The frontend generates a nonce challenge and requests the wallet to sign it with `signAuthEntry` or signature of challenge message.
   - The backend validates the cryptographic signature against the user's public address and returns an access token and refresh token.
2. **Token Refresh Endpoint**:
   - `POST /api/v1/auth/refresh` rotates an unexpired refresh token into a new access/refresh pair.

### WebSocket Authentication

- **Production**: WebSocket handshake requires the `Authorization: Bearer <token>` header or `Sec-WebSocket-Protocol` ticket exchange.
- **Development/Test**: The query parameter `?token=<jwt_access_token>` is accepted for convenience, but is rejected in production to prevent token leakage in proxy logs.

---

## Rate Limiting Details

PrediFi implements an in-memory token bucket sliding window rate limiter (`governor` / `tower_governor`) keyed on the client's public IP address (accounting for `X-Forwarded-For` and `X-Real-IP` reverse proxy headers).

### Rate Limit Tiers

| Tier | Capacity (Burst) | Window / Period | Max Sustained Rate | Applied Routes |
|------|------------------|-----------------|--------------------|----------------|
| **Light** | 120 tokens | 60 seconds | 2.0 req/s | `GET /`, `GET /health`, `GET /api/v1/fees`, `GET /api/v1/prices` |
| **Read** | 60 tokens | 60 seconds | 1.0 req/s | `GET /api/v1/pools`, `GET /api/v1/pools/{id}`, `GET /api/v1/stats`, `GET /api/v1/leaderboard`, `GET /api/v1/tags`, `GET /api/v1/referrals/*`, `GET /api/v1/markets/{id}/predictions` |
| **User** | 30 tokens | 60 seconds | 0.5 req/s | `GET /api/v1/users/{address}/history`, `GET /api/v1/users/{address}/predictions`, `GET /api/v1/users/{address}/profile`, `GET /api/v1/users/{address}/interests`, `GET /api/v1/notifications/{address}` |
| **Write** | 20 tokens | 60 seconds | 0.33 req/s | `POST /api/v1/indexer/*`, `PATCH /api/v1/pools/{id}/tags` |
| **Token** | 10 tokens | 60 seconds | 0.16 req/s | `POST /api/v1/auth/refresh`, `GET /api/v1/ws` (handshake) |
| **Admin** | Exempt / Custom | - | Restricted | `PATCH /api/v1/admin/log-level` (requires admin JWT role) |

### Rate Limit Response Headers

When rate limits are evaluated, the following HTTP headers are included in responses:

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 42
X-RateLimit-Reset: 1773996460
```

When the quota is exceeded, the server returns HTTP `429 Too Many Requests`:

```http
HTTP/1.1 429 Too Many Requests
Content-Type: application/json
Retry-After: 18

{
  "status": "error",
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Retry after 18 seconds.",
    "request_id": "c1f7a0b2-4d1e-42c9-9a28-3e1987d601b3"
  }
}
```

---

## WebSocket Subscription Documentation

The WebSocket broadcast feed delivers real-time market updates, prediction placements, and resolution alerts to connected web clients and automated bots.

### Connection Endpoint

```
GET /api/v1/ws HTTP/1.1
Host: api.predifi.com
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
Authorization: Bearer <jwt_access_token>
```

### Connection Query Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `address` | string | No | Filter events to only those where `user_address` matches this Stellar account (must be the JWT subject). |
| `pool_id` | integer | No | Subscribe to one pool (legacy; merged with `pool_ids`). |
| `pool_ids` | string | No | Comma-separated pool IDs to subscribe to, e.g. `1042,1043`. Max 100 pools per connection. |
| `events` | string | No | Comma-separated event types to receive, e.g. `prediction_placed,pool_resolved`. |
| `batch_ms` | integer | No | Batching window, `1`–`1000` ms. Matching events are buffered and sent as one `batch` frame. Omit (or `0`) for one frame per event. |
| `batch_max` | integer | No | Flush a batch once it holds this many events (default `50`, max `500`). |
| `compress` | string | No | `deflate` — frames of 1 KiB or more are zlib-compressed and sent as **binary** frames. |
| `token` | string | No | Bearer token (permitted only in non-production environments). |

> **Tip**: If neither `address` nor any pool is supplied, the client receives all protocol broadcast events across the platform (recommended for global live activity feeds and dashboards). Invalid parameters are rejected with `400` before the upgrade.

### Changing Subscriptions at Runtime

Send JSON control frames on the open socket; each is answered with the current subscription:

```json
{"action": "subscribe",   "pool_ids": [1044, 1045]}
{"action": "unsubscribe", "pool_ids": [1042]}
{"action": "list"}
```

```json
{"type": "subscriptions", "pool_ids": [1043, 1044, 1045], "events": []}
```

An empty `pool_ids` set means "every pool". Unknown frames get `{"type":"error","message":"..."}`. Control frames count towards the per-connection message rate limit (10 per 10 s).

### Batched and Compressed Delivery

With `batch_ms`, events arrive wrapped in a batch envelope (the inner events are exactly the per-event payloads below):

```json
{"type": "batch", "count": 2, "events": [{"type": "prediction_placed", "...": "..."}, {"type": "prediction_placed", "...": "..."}]}
```

With `compress=deflate`, any frame of at least 1 KiB is sent as a binary zlib stream; smaller frames stay text. In browsers:

```typescript
ws.binaryType = "arraybuffer";
ws.onmessage = async (msg) => {
  const text = typeof msg.data === "string"
    ? msg.data
    : await new Response(new Blob([msg.data]).stream().pipeThrough(new DecompressionStream("deflate"))).text();
  const frame = JSON.parse(text);
  const events = frame.type === "batch" ? frame.events : [frame];
  // ...
};
```

### Connection Limits & Safeguards

- **Handshake Rate Limit**: 10 connection attempts per 60 seconds per IP address (`Token` tier).
- **Concurrent Connections**: 10,000 maximum active clients per server instance.
- **Message Size**: Max 1 MB (`1,048,576 bytes`) per frame; batches are flushed before exceeding it.
- **Channel Buffer**: 4,096 events. A consumer that falls further behind receives `{"type":"lagged","missed":N}` and should resynchronise over REST.

### WebSocket Event Schemas

All messages dispatched by the server are JSON objects containing a `"type"` discriminator:

#### 1. `prediction_placed`
Dispatched immediately when a prediction is indexed from Soroban contract events:

```json
{
  "type": "prediction_placed",
  "pool_id": 1042,
  "user_address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
  "outcome": 1,
  "amount": 500000000,
  "created_at": "2026-10-16T10:30:00Z"
}
```

#### 2. `pool_created`
Dispatched when a new market pool is initialized:

```json
{
  "type": "pool_created",
  "pool_id": 1043,
  "creator": "GA7N6WZ4Z2M4K266M7P5B3NQQ3J43C2IXXD6VBL75276W4P6NRPPQ5X3",
  "end_time": 1774050000,
  "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "category": "Crypto",
  "description": "Will XLM exceed $0.50 by November 2026?",
  "created_at": "2026-10-16T11:00:00Z"
}
```

#### 3. `pool_resolved`
Dispatched when a pool resolution consensus is registered:

```json
{
  "type": "pool_resolved",
  "pool_id": 1042,
  "result": 1,
  "resolvers": [
    "GCXPBNCGYPZG456P74CFFV4V7TTR2VYF4PQWZXD6A53H2G64K57QORAC"
  ],
  "resolved_at": "2026-10-16T11:30:00Z"
}
```

### WebSocket Client Implementations

#### Browser / TypeScript Example

```typescript
const token = "YOUR_JWT_TOKEN";
const poolId = 1042;
const socket = new WebSocket(`wss://api.predifi.com/api/v1/ws?pool_id=${poolId}`);

socket.addEventListener('open', () => {
  console.log('[WS] Connected to PrediFi stream');
});

socket.addEventListener('message', (event: MessageEvent) => {
  try {
    const data = JSON.parse(event.data);
    switch (data.type) {
      case 'prediction_placed':
        console.log(`[Prediction] New stake: ${data.amount / 1e7} XLM on outcome ${data.outcome}`);
        break;
      case 'pool_resolved':
        console.log(`[Resolution] Pool ${data.pool_id} resolved with outcome ${data.result}`);
        break;
      default:
        console.log('[WS Message]', data);
    }
  } catch (err) {
    console.error('[WS Parse Error]', err);
  }
});

socket.addEventListener('close', (event) => {
  console.warn(`[WS Closed] code=${event.code}, reason=${event.reason}`);
});
```

#### Python (`websockets` / `websocket-client`) Example

```python
import json
import websocket

def on_message(ws, message):
    payload = json.loads(message)
    print("Received event:", payload)

def on_open(ws):
    print("WebSocket connection established")

if __name__ == "__main__":
    ws_url = "wss://api.predifi.com/api/v1/ws?address=GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635"
    headers = ["Authorization: Bearer YOUR_ACCESS_TOKEN"]
    
    ws = websocket.WebSocketApp(
        ws_url,
        header=headers,
        on_message=on_message,
        on_open=on_open
    )
    ws.run_forever()
```

---

## API Endpoints Reference

### Health & System Diagnostics

#### `GET /health`
Liveness and comprehensive dependency check. Queries Postgres with `SELECT 1`, performs HTTP `getHealth` on Stellar RPC with exponential backoff, checks Redis connectivity, and validates `PriceCache`.

- **Tag**: `health`
- **Rate Limit**: Light tier (120 req / 60s)
- **Auth**: None

##### Response (200 OK - Healthy)
```json
{
  "status": "healthy",
  "service": "predifi-api",
  "version": "1.0.0",
  "dependencies": {
    "db": "ok",
    "rpc": "ok",
    "redis": "ok",
    "price_cache": "ok"
  },
  "errors": {
    "db": null,
    "rpc": null,
    "redis": null,
    "price_cache": null
  }
}
```

##### Response (503 Service Unavailable - Degraded)
```json
{
  "status": "unhealthy",
  "service": "predifi-api",
  "version": "1.0.0",
  "dependencies": {
    "db": "unreachable",
    "rpc": "ok",
    "redis": "ok",
    "price_cache": "ok"
  },
  "errors": {
    "db": "connection to server at 'postgres:5432' failed: Connection refused",
    "rpc": null,
    "redis": null,
    "price_cache": null
  }
}
```

#### `GET /live`
Kubernetes liveness probe. Returns HTTP 200 as long as the HTTP listener is operational.

#### `GET /ready`
Kubernetes readiness probe. Returns HTTP 200 when ready to accept traffic.

#### `GET /metrics`
Prometheus metrics scrape target exporting request counters, latencies, active connections, and cache hit ratios.

---

### Prediction Market Pools

#### `GET /api/v1/pools`
Retrieve a paginated list of prediction market pools filtered by category, status, and sorted by volume or end time.

- **Tag**: `pools`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `sort_by` | string | No | `new` | Ordering: `new`, `popular` (total stake), `ending_soon`. |
| `category` | string | No | - | Category filter e.g. `Crypto`, `Sports`, `Politics`, `Economics`. |
| `status` | string | No | `active` | Filter by state: `active`, `closed`, `settled`. |
| `limit` | integer | No | 20 | Page size (1 to 100). |
| `offset` | integer | No | 0 | Number of rows to skip. |

##### Response (200 OK)
```json
{
  "pools": [
    {
      "pool_id": 1042,
      "name": "Stellar Protocol 23 Upgrade Success",
      "category": "Crypto",
      "total_stake": 15000000000,
      "end_time": 1774050000,
      "created_at": "2026-10-15T08:00:00Z",
      "state": "active",
      "creator": "GA7N6WZ4Z2M4K266M7P5B3NQQ3J43C2IXXD6VBL75276W4P6NRPPQ5X3",
      "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
      "result": null
    }
  ],
  "limit": 20,
  "offset": 0,
  "status": "active",
  "sort_by": "new"
}
```

#### `GET /api/v1/pools/{pool_id}`
Retrieve granular details of an individual prediction pool, including real-time outcome stake distributions and implied odds.

- **Tag**: `pools`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Path Parameters
| Parameter | Type | Description |
|-----------|------|-------------|
| `pool_id` | integer | On-chain pool identifier (u64). |

##### Response (200 OK)
```json
{
  "pool_id": 1042,
  "name": "Stellar Protocol 23 Upgrade Success",
  "category": "Crypto",
  "total_stake": 15000000000,
  "end_time": 1774050000,
  "created_at": "2026-10-15T08:00:00Z",
  "state": "active",
  "creator": "GA7N6WZ4Z2M4K266M7P5B3NQQ3J43C2IXXD6VBL75276W4P6NRPPQ5X3",
  "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "result": null,
  "odds": [
    {
      "outcome": 0,
      "stake": 10000000000,
      "odds": 1.5
    },
    {
      "outcome": 1,
      "stake": 5000000000,
      "odds": 3.0
    }
  ]
}
```

##### Response (404 Not Found)
```json
{
  "status": "error",
  "error": {
    "code": "NOT_FOUND",
    "message": "Pool with ID 999999 not found",
    "request_id": "4b68e9e1-64d8-4f51-b844-325bdfa91b24"
  }
}
```

---

### Protocol Statistics & Fees

#### `GET /api/v1/stats`
Returns system-wide aggregated metrics, including total value locked (TVL) in stroops, aggregate prediction count, and pool count.

- **Tag**: `stats`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Response (200 OK)
```json
{
  "total_value_locked": 250000000000,
  "total_bets": 48250,
  "total_pools": 312
}
```

#### `GET /api/v1/fees`
Returns current on-chain fee configuration parameters.

- **Tag**: `stats`
- **Rate Limit**: Light tier (120 req / 60s)
- **Auth**: None

##### Response (200 OK)
```json
{
  "treasury_fee_bps": 100,
  "referral_fee_bps": 5000
}
```
*Notes: `treasury_fee_bps: 100` represents 1.00% protocol fee. `referral_fee_bps: 5000` indicates 50% of the protocol fee is distributed to referring accounts.*

#### `GET /api/v1/prices`
Returns the latest cached oracle asset prices retrieved from trusted price feeds.

- **Tag**: `stats`
- **Rate Limit**: Light tier (120 req / 60s)
- **Auth**: None

##### Response (200 OK)
```json
{
  "prices": {
    "XLM/USD": 0.425,
    "BTC/USD": 88400.0,
    "ETH/USD": 3850.25
  },
  "updated_at": "2026-10-16T11:45:00Z"
}
```

---

### Leaderboards & User Rankings

#### `GET /api/v1/leaderboard`
Query platform leaderboards ranked by total betting volume or aggregate winnings.

- **Tag**: `leaderboard`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `rank_by` | string | No | `volume` | Ranking mode: `volume` or `winnings`. |
| `limit` | integer | No | 20 | Page size (1 to 100). |
| `offset` | integer | No | 0 | Page offset. |

##### Response (200 OK - Ranked by Volume)
```json
{
  "leaderboard": [
    {
      "user_address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
      "total_volume": 45000000000,
      "prediction_count": 182,
      "rank": 1
    }
  ],
  "rank_by": "volume",
  "limit": 20,
  "offset": 0
}
```

---

### Predictions & User History

#### `GET /api/v1/users/{address}/history`
Fetch raw chronological prediction history for a specific Stellar address.

- **Tag**: `predictions`
- **Rate Limit**: User tier (30 req / 60s)
- **Auth**: None (Public)

##### Path Parameters
- `address`: Stellar G... account address.

##### Response (200 OK)
```json
{
  "address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
  "predictions": [
    {
      "pool_id": 1042,
      "pool_name": "Stellar Protocol 23 Upgrade Success",
      "pool_result": "1",
      "outcome": 1,
      "amount": 500000000,
      "created_at": "2026-10-15T09:15:00Z"
    }
  ],
  "limit": 20,
  "offset": 0
}
```

#### `GET /api/v1/users/{address}/predictions`
Fetch enhanced predictions augmented with current pool state and user win status.

- **Tag**: `predictions`
- **Rate Limit**: User tier (30 req / 60s)
- **Auth**: None

##### Response (200 OK)
```json
{
  "address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
  "predictions": [
    {
      "prediction_id": 8421,
      "pool_id": 1042,
      "pool_name": "Stellar Protocol 23 Upgrade Success",
      "pool_category": "Crypto",
      "pool_state": "resolved",
      "pool_total_stake": 15000000000,
      "pool_result": "1",
      "user_outcome": 1,
      "user_amount": 500000000,
      "is_winning_outcome": true
    }
  ],
  "limit": 20,
  "offset": 0,
  "total_predictions": 1
}
```

#### `GET /api/v1/markets/{market_id}/predictions`
Retrieve cursor-paginated list of predictions placed on a specific market pool.

- **Tag**: `predictions`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Query Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `after` | integer | No | Stable row ID cursor value from previous `next_cursor`. |
| `limit` | integer | No | Page size (1 to 100, default 20). |

##### Response (200 OK)
```json
{
  "market_id": 1042,
  "predictions": [
    {
      "id": 8421,
      "pool_id": 1042,
      "user_address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
      "outcome": 1,
      "amount": 500000000,
      "created_at": "2026-10-15T09:15:00Z"
    }
  ],
  "total": 45,
  "limit": 20,
  "next_cursor": 8440
}
```

---

### Referrals & Rewards

#### `GET /api/v1/referrals/{address}`
Retrieve aggregated referral volume and earnings for an affiliate referrer.

- **Tag**: `referrals`
- **Rate Limit**: Read tier (60 req / 60s)
- **Auth**: None

##### Response (200 OK)
```json
{
  "referrer": "GA7N6WZ4Z2M4K266M7P5B3NQQ3J43C2IXXD6VBL75276W4P6NRPPQ5X3",
  "total_earned": 250000000,
  "pools": [
    {
      "pool_id": 1042,
      "pool_name": "Stellar Protocol 23 Upgrade Success",
      "total_earned": 150000000,
      "referral_count": 18
    }
  ]
}
```

#### `GET /api/v1/users/{address}/referrals`
Per-pool referral earnings breakdown matching `ReferralEarningsResponse` schema.

---

### Indexer Ingestion (Internal)

These endpoints ingest events emitted by the Soroban contract. In production, access is restricted to authenticated internal indexer microservices.

#### `POST /api/v1/indexer/pool-created`
- **Tag**: `indexer`
- **Rate Limit**: Write tier (20 req / 60s)
- **Auth**: Required (Indexer service role)

##### Request Body
```json
{
  "pool_id": 1043,
  "creator": "GA7N6WZ4Z2M4K266M7P5B3NQQ3J43C2IXXD6VBL75276W4P6NRPPQ5X3",
  "end_time": 1774050000,
  "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "category": "Crypto",
  "description": "Will XLM exceed $0.50 by November 2026?"
}
```

##### Response (200 OK)
```json
{
  "status": "ok",
  "pool_id": 1043
}
```

#### `POST /api/v1/indexer/prediction-placed`
- **Tag**: `indexer`
- **Rate Limit**: Write tier (20 req / 60s)
- **Auth**: Required (Indexer service role)

##### Request Body
```json
{
  "pool_id": 1043,
  "user_address": "GBZXN7PIRZGNMHGA728JAGEL2OC4AHFAXE2XZ2VY5UZWFEXU272K5635",
  "outcome": 1,
  "amount": 250000000
}
```

##### Response (200 OK)
```json
{
  "status": "ok",
  "pool_id": 1043
}
```

---

### Tags, Notifications & Templates

- `GET /api/v1/tags`: List active searchable pool tags.
- `PATCH /api/v1/pools/{id}/tags`: Update tag list for a pool (requires creator auth).
- `GET /api/v1/users/{address}/profile`: Fetch comprehensive user profile statistics.
- `GET /api/v1/users/{address}/interests` / `PUT /api/v1/users/{address}/interests`: Get/set user category interest preferences.
- `GET /api/v1/notifications/{address}`: List pending user activity notifications.
- `POST /api/v1/notifications/{address}/read`: Mark notifications as acknowledged.
- `GET /api/v1/pool-templates` / `POST /api/v1/pool-templates`: Manage recurring prediction templates.

---

## Request and Response Examples

### cURL Examples

#### 1. Query Pools with Filters and Sorting
```bash
curl -s -X GET "https://api.predifi.com/v1/pools?category=Crypto&sort_by=popular&limit=5" \
  -H "Accept: application/json"
```

#### 2. Query Pool with Live Odds
```bash
curl -s -X GET "https://api.predifi.com/v1/pools/1042" \
  -H "Accept: application/json"
```

#### 3. Cursor Pagination for Predictions
```bash
curl -s -X GET "https://api.predifi.com/v1/markets/1042/predictions?limit=10&after=8421" \
  -H "Accept: application/json"
```

#### 4. Connect to Real-Time WebSocket via `wscat`
```bash
wscat -c "wss://api.predifi.com/api/v1/ws?pool_id=1042" \
  -H "Authorization: Bearer <jwt_access_token>"
```

---

## Error Handling & Error Envelopes

All errors returned by the API conform to a uniform JSON envelope containing a machine-readable code, human-readable description, and a UUID `request_id` for distributed log tracing.

```json
{
  "status": "error",
  "error": {
    "code": "INVALID_INPUT",
    "message": "Limit must be between 1 and 100",
    "request_id": "9d55ec6b-dfae-42b7-a369-0df8549f05d2"
  }
}
```

### Standard HTTP Status & Error Code Matrix

| HTTP Status | String Code | Cause |
|-------------|-------------|-------|
| `400 Bad Request` | `INVALID_INPUT` | Parameter constraint violation, malformed address, or invalid pagination range. |
| `401 Unauthorized` | `UNAUTHORIZED` | Missing or invalid Bearer JWT on protected route. |
| `403 Forbidden` | `FORBIDDEN` | Caller lacks the administrative or creator role. |
| `404 Not Found` | `NOT_FOUND` | Specified pool, prediction, or user address was not found. |
| `409 Conflict` | `CONFLICT` | Unique key or state machine conflict (e.g. duplicate pool index). |
| `429 Too Many Requests` | `RATE_LIMIT_EXCEEDED` | Exceeded token bucket quota for the route tier. |
| `500 Internal Server Error` | `INTERNAL_ERROR` | Unhandled internal exception or database execution error. |
| `503 Service Unavailable` | `SERVICE_UNAVAILABLE` | Database connection lost or RPC provider unreachable. |

---

## Client SDK Generation

Because PrediFi provides a strictly typed OpenAPI 3.1 schema, client SDKs can be generated automatically for TypeScript, Python, and Rust.

### TypeScript / Next.js SDK

Using `openapi-typescript` and `openapi-fetch`:

```bash
# 1. Generate TypeScript interfaces directly from the backend spec
npx openapi-typescript https://api.predifi.com/api-docs/openapi.json \
  -o frontend/lib/api/schema.d.ts

# 2. Or generate from local file
npx openapi-typescript backend/openapi.json \
  -o frontend/lib/api/schema.d.ts
```

Client usage in Next.js:

```typescript
import createClient from "openapi-fetch";
import type { paths } from "@/lib/api/schema";

const client = createClient<paths>({ baseUrl: "https://api.predifi.com/v1" });

const { data, error } = await client.GET("/pools/{pool_id}", {
  params: { path: { pool_id: 1042 } },
});

if (data) {
  console.log("Pool odds:", data.odds);
}
```

### Python SDK

Using `openapi-python-client`:

```bash
pip install openapi-python-client
openapi-python-client generate --url https://api.predifi.com/api-docs/openapi.json --output-path ./predifi-sdk
```

---

**Document Maintained By**: Web3Novalabs Core Engineering  
**Source Code References**: [openapi.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/openapi.rs), [ws.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/ws.rs), [rate_limit.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/rate_limit.rs), [server.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/server.rs)
