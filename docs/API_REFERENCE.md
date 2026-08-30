# PrediFi API Reference

**Version**: 1.0.0  
**Specification**: OpenAPI 3.0  
**Base URL**: `https://api.predifi.com/v1`  
**Documentation URL**: `/api-docs/openapi.json` (raw spec)  

---

## Table of Contents

1. [Authentication](#authentication)
2. [Rate Limiting](#rate-limiting)
3. [WebSocket Subscriptions](#websocket-subscriptions)
4. [HTTP Endpoints](#http-endpoints)
   - [Health Check](#health-check)
   - [Pools](#pools)
   - [Stats](#stats)
   - [Leaderboard](#leaderboard)
   - [Predictions](#predictions)
   - [Referrals](#referrals)
   - [Indexer](#indexer)
5. [Request/Response Examples](#requestresponse-examples)
6. [Error Responses](#error-responses)

---

## Authentication

All protected endpoints require a valid JWT access token in the `Authorization` header:

```http
Authorization: Bearer <jwt_access_token>
```

### JWT Token Structure

Tokens are issued by the authentication service and contain:

```json
{
  "sub": "GD... wallet address",
  "type": "access",
  "exp": 1234567890,
  "iat": 1234567890,
  "key_version": 0
}
```

### Obtaining a Token

1. User connects their wallet
2. Client generates and signs a message
3. Backend verifies signature and issues JWT
4. Token is stored client-side (recommended: HttpOnly cookie)

### Token Expiration

- **Access tokens**: 1 hour (3600 seconds)
- **Refresh tokens**: 7 days (604800 seconds)

### WebSocket Authentication

WebSocket connections require the same JWT token:

```http
GET /api/v1/ws?address=GD... HTTP/1.1
Authorization: Bearer <jwt_access_token>
Origin: https://app.predifi.com
```

**Security Note**: JWT tokens in query parameters are only accepted in non-production environments (dev/test). Production requires the `Authorization` header to prevent token leakage in logs.

---

## Rate Limiting

PrediFi implements tiered rate limiting using a token bucket algorithm. All limits are per IP address unless otherwise specified.

### Rate Limit Tiers

| Tier | Burst Size | Period | Sustained Rate | Endpoints |
|------|------------|--------|----------------|-----------|
| **Light** | 120 | 60s | 2 req/s | `/health`, `/fees`, `/prices` |
| **Read** | 60 | 60s | 1 req/s | `/pools`, `/stats`, `/leaderboard` |
| **User** | 30 | 60s | 0.5 req/s | `/users/{address}/history` |
| **Write** | 20 | 60s | 0.33 req/s | `/indexer/*` |
| **Token** | 10 | 60s | 0.16 req/s | `/refresh-token` |
| **WebSocket** | 10 | 10s | 1 msg/s | `/api/v1/ws` |

### HTTP Response Headers

Rate-limited responses include:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 30
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1672531200
```

### Error Response

```json
{
  "error": "RATE_LIMIT_EXCEEDED",
  "message": "Rate limit exceeded. Retry after 30 seconds."
}
```

### WebSocket Rate Limiting

WebSocket connections have separate rate limiting:
- **Inbound messages**: 10 messages per 10 seconds
- **Message size**: 1 MB maximum
- **Active connections**: 10,000 maximum per instance

---

## WebSocket Subscriptions

### Connection Endpoint

```
GET /api/v1/ws?address=<wallet>&pool_id=<pool_id> HTTP/1.1
```

### Query Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `address` | No | Filter events by wallet address |
| `pool_id` | No | Filter events by pool ID |
| `token` | No | JWT token (dev only - ignored in production) |

### Authentication

```http
Authorization: Bearer <jwt_access_token>
Origin: https://app.predifi.com
```

### Event Types

#### 1. Prediction Placed
```json
{
  "type": "prediction_placed",
  "pool_id": 123,
  "user_address": "GD...",
  "outcome": 1,
  "amount": 10000000,
  "created_at": "2026-01-15T10:30:00Z"
}
```

#### 2. Pool Created
```json
{
  "type": "pool_created",
  "pool_id": 123,
  "creator": "GD...",
  "end_time": 1673781600,
  "token": "CD...",
  "category": "Crypto",
  "description": "ETH/USD Prediction",
  "created_at": "2026-01-15T10:00:00Z"
}
```

#### 3. Pool Resolved
```json
{
  "type": "pool_resolved",
  "pool_id": 123,
  "result": 1,
  "resolvers": ["GD1...", "GD2..."],
  "resolved_at": "2026-01-15T12:00:00Z"
}
```

### Event Filtering

#### By Wallet Address
```http
GET /api/v1/ws?address=GDABC... HTTP/1.1
Authorization: Bearer <token>
```
Only events for the specified wallet are delivered.

#### By Pool ID
```http
GET /api/v1/ws?pool_id=123 HTTP/1.1
Authorization: Bearer <token>
```
Only events for the specified pool are delivered.

#### No Filter (All Events)
```http
GET /api/v1/ws HTTP/1.1
Authorization: Bearer <token>
```
All events are delivered (useful for dashboards).

### Client Example (JavaScript)

```javascript
const socket = new WebSocket('wss://api.predifi.com/api/v1/ws?address=GD...');

socket.addEventListener('open', () => {
  console.log('WebSocket connected');
});

socket.addEventListener('message', (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
  
  if (data.type === 'prediction_placed') {
    console.log(`New prediction for pool ${data.pool_id}`);
  }
});

socket.addEventListener('error', (error) => {
  console.error('WebSocket error:', error);
});

socket.addEventListener('close', () => {
  console.log('WebSocket disconnected');
});
```

### Client Example (Python)

```python
import websocket
import json

def on_message(ws, message):
    data = json.loads(message)
    print(f"Received: {data}")

def on_error(ws, error):
    print(f"Error: {error}")

def on_close(ws, close_status_code, close_msg):
    print(f"Closed: {close_status_code} - {close_msg}")

def on_open(ws):
    print("Connection opened")

if __name__ == "__main__":
    ws = websocket.WebSocketApp(
        "wss://api.predifi.com/api/v1/ws?address=GD...",
        header=["Authorization: Bearer YOUR_JWT_TOKEN"],
        on_open=on_open,
        on_message=on_message,
        on_error=on_error,
        on_close=on_close
    )
    ws.run_forever()
```

### Connection Limits

- **Max connections per IP**: 10 handshake attempts per 60 seconds
- **Max active connections**: 10,000 per server instance
- **Message queue**: 256 messages per connection (older messages dropped if queue full)

---

## HTTP Endpoints

### Health Check

#### GET `/health`

Check service health and dependency status.

**Authentication**: Not required

**Rate Limit**: Light tier (120 req/60s)

#### Response

```json
{
  "status": "healthy",
  "service": "predifi-api",
  "version": "1.0.0",
  "dependencies": {
    "db": "healthy",
    "rpc": "healthy",
    "redis": "healthy",
    "price_cache": "healthy"
  },
  "errors": {
    "db": null,
    "rpc": null,
    "redis": null,
    "price_cache": null
  }
}
```

**Status Codes**:
- `200`: All dependencies healthy
- `503`: One or more dependencies unavailable

---

### Pools

#### GET `/api/v1/pools`

List all prediction markets with pagination.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Query Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `sort_by` | string | `popular`, `ending_soon`, `new` (default: `new`) |
| `category` | string | Filter by category (e.g., `Sports`, `Crypto`) |
| `status` | string | `active`, `closed`, `settled` (default: `active`) |
| `limit` | integer | Max results (default: 20, max: 100) |
| `offset` | integer | Pagination offset (default: 0) |

#### Response

```json
{
  "pools": [
    {
      "pool_id": 123,
      "name": "ETH/USD Prediction",
      "category": "Crypto",
      "total_stake": 500000000,
      "end_time": 1673781600,
      "created_at": "2026-01-15T10:00:00Z",
      "state": "active",
      "creator": "GD...",
      "token": "CD...",
      "result": null
    }
  ],
  "limit": 20,
  "offset": 0,
  "status": "success",
  "sort_by": "new"
}
```

#### GET `/api/v1/pools/{pool_id}`

Get a specific pool with live odds.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Path Parameters**:
- `pool_id`: On-chain pool identifier (integer)

#### Response

```json
{
  "pool": {
    "pool_id": 123,
    "name": "ETH/USD Prediction",
    "category": "Crypto",
    "total_stake": 500000000,
    "end_time": 1673781600,
    "created_at": "2026-01-15T10:00:00Z",
    "state": "active",
    "creator": "GD...",
    "token": "CD...",
    "result": null
  },
  "odds": [
    {
      "outcome": 0,
      "stake": 250000000,
      "odds": 2.0
    },
    {
      "outcome": 1,
      "stake": 250000000,
      "odds": 2.0
    }
  ]
}
```

---

### Stats

#### GET `/api/v1/stats`

Get protocol-wide aggregate statistics.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

#### Response

```json
{
  "total_value_locked": 5000000000,
  "total_bets": 12500,
  "total_pools": 150
}
```

#### GET `/api/v1/fees`

Get current protocol fee configuration.

**Authentication**: Not required

**Rate Limit**: Light tier (120 req/60s)

#### Response

```json
{
  "treasury_fee_bps": 200,
  "referral_fee_bps": 5000
}
```

**Fee Values** (basis points):
- `treasury_fee_bps`: 200 = 2% protocol fee
- `referral_fee_bps`: 5000 = 50% of protocol fee to referrers

#### GET `/api/v1/prices`

Get latest cached asset prices.

**Authentication**: Not required

**Rate Limit**: Light tier (120 req/60s)

#### Response

```json
{
  "prices": {
    "ETH/USD": 3500.50,
    "BTC/USD": 72000.00,
    "SOL/USD": 145.75
  },
  "timestamp": "2026-01-15T10:30:00Z"
}
```

---

### Leaderboard

#### GET `/api/v1/leaderboard`

Get user rankings.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Query Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `rank_by` | string | `volume` (default) or `winnings` |
| `limit` | integer | Max results (default: 20, max: 100) |
| `offset` | integer | Pagination offset (default: 0) |

#### Response (Volume Ranking)

```json
{
  "leaderboard": [
    {
      "user_address": "GD...",
      "total_volume": 500000000,
      "prediction_count": 150,
      "rank": 1
    }
  ],
  "rank_by": "volume",
  "limit": 20,
  "offset": 0
}
```

#### Response (Winnings Ranking)

```json
{
  "leaderboard": [
    {
      "user_address": "GD...",
      "total_winnings": 250000000,
      "winning_predictions": 75,
      "total_predictions": 150,
      "win_rate": 0.5,
      "rank": 1
    }
  ],
  "rank_by": "winnings",
  "limit": 20,
  "offset": 0
}
```

---

### Predictions

#### GET `/api/v1/users/{address}/history`

Get user's prediction history.

**Authentication**: Not required

**Rate Limit**: User tier (30 req/60s)

**Path Parameters**:
- `address`: Stellar account address (G...)

**Query Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | integer | Max results (default: 20, max: 100) |
| `offset` | integer | Pagination offset (default: 0) |

#### Response

```json
{
  "address": "GD...",
  "predictions": [
    {
      "pool_id": 123,
      "pool_name": "ETH/USD Prediction",
      "pool_result": "1",
      "outcome": 1,
      "amount": 10000000,
      "created_at": "2026-01-15T10:30:00Z"
    }
  ],
  "limit": 20,
  "offset": 0
}
```

#### GET `/api/v1/users/{address}/predictions`

Get enhanced predictions with current pool status.

**Authentication**: Not required

**Rate Limit**: User tier (30 req/60s)

**Path Parameters**:
- `address`: Stellar account address (G...)

#### Response

```json
{
  "address": "GD...",
  "predictions": [
    {
      "prediction_id": 456,
      "pool_id": 123,
      "pool_name": "ETH/USD Prediction",
      "pool_category": "Crypto",
      "pool_state": "active",
      "pool_total_stake": 500000000,
      "pool_result": null,
      "user_outcome": 1,
      "user_amount": 10000000,
      "is_winning_outcome": null
    }
  ],
  "limit": 20,
  "offset": 0,
  "total_predictions": 150
}
```

#### GET `/api/v1/markets/{market_id}/predictions`

Get predictions for a specific market with cursor pagination.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Path Parameters**:
- `market_id`: On-chain pool identifier (integer)

**Query Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `after` | integer | Cursor from previous page's `next_cursor` |
| `limit` | integer | Page size (1-100, default: 20) |

#### Response

```json
{
  "market_id": 123,
  "predictions": [
    {
      "id": 456,
      "pool_id": 123,
      "user_address": "GD...",
      "outcome": 1,
      "amount": 10000000,
      "created_at": "2026-01-15T10:30:00Z"
    }
  ],
  "total": 1000,
  "limit": 20,
  "next_cursor": 476
}
```

---

### Referrals

#### GET `/api/v1/referrals/{address}`

Get referral summary for an address.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Path Parameters**:
- `address`: Stellar referrer address (G...)

#### Response

```json
{
  "referrer": "GD...",
  "total_earned": 25000000,
  "pools": [
    {
      "pool_id": 123,
      "pool_name": "ETH/USD Prediction",
      "total_earned": 15000000,
      "referral_count": 25
    },
    {
      "pool_id": 456,
      "pool_name": "BTC/USD Prediction",
      "total_earned": 10000000,
      "referral_count": 18
    }
  ]
}
```

#### GET `/api/v1/users/{address}/referrals`

Get per-pool referral earnings.

**Authentication**: Not required

**Rate Limit**: Read tier (60 req/60s)

**Path Parameters**:
- `address`: Stellar referrer address (G...)

#### Response

```json
{
  "referrer": "GD...",
  "total_earned": 25000000,
  "pools": [
    {
      "pool_id": 123,
      "pool_name": "ETH/USD Prediction",
      "total_earned": 15000000,
      "referral_count": 25
    }
  ]
}
```

---

### Indexer

#### POST `/api/v1/indexer/pool-created`

Ingest a new pool event (internal endpoint).

**Authentication**: Required (JWT with indexer role)

**Rate Limit**: Write tier (20 req/60s)

**Request Body**

```json
{
  "pool_id": 123,
  "creator": "GD...",
  "end_time": 1673781600,
  "token": "CD...",
  "category": "Crypto",
  "description": "ETH/USD Prediction"
}
```

#### Response

```json
{
  "status": "success",
  "pool_id": 123
}
```

#### POST `/api/v1/indexer/prediction-placed`

Ingest a prediction event (internal endpoint).

**Authentication**: Required (JWT with indexer role)

**Rate Limit**: Write tier (20 req/60s)

**Request Body**

```json
{
  "pool_id": 123,
  "user_address": "GD...",
  "outcome": 1,
  "amount": 10000000
}
```

#### Response

```json
{
  "status": "success",
  "pool_id": 123
}
```

---

## Request/Response Examples

### curl Examples

#### List Active Pools

```bash
curl -X GET "https://api.predifi.com/api/v1/pools?sort_by=popular&category=Crypto" \
  -H "Accept: application/json"
```

#### Get Pool Details

```bash
curl -X GET "https://api.predifi.com/api/v1/pools/123" \
  -H "Accept: application/json"
```

#### Get User Prediction History

```bash
curl -X GET "https://api.predifi.com/api/v1/users/GD.../history?limit=50" \
  -H "Accept: application/json"
```

#### Subscribe to WebSocket

```bash
wscat -c "wss://api.predifi.com/api/v1/ws?address=GD..." \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### JavaScript Examples

#### Fetch Pools

```javascript
const response = await fetch('https://api.predifi.com/api/v1/pools?sort_by=new');
const data = await response.json();
console.log(data.pools);
```

#### Place a Prediction (via smart contract)

```javascript
// PrediFi uses Stellar/Soroban smart contracts for predictions
// API only provides read operations
// Use the smart contract directly for write operations

const poolId = 123;
const outcome = 1;
const amount = 10000000; // 10 XLM in stroops

// Contract call would go here
```

#### WebSocket Subscription

```javascript
const ws = new WebSocket('wss://api.predifi.com/api/v1/ws?address=GD...');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'prediction_placed') {
    console.log(`New prediction: ${data.amount} stroops on pool ${data.pool_id}`);
  }
};
```

### Python Examples

#### Get Protocol Stats

```python
import requests

response = requests.get('https://api.predifi.com/api/v1/stats')
data = response.json()
print(f"Total Value Locked: {data['total_value_locked']}")
print(f"Total Pools: {data['total_pools']}")
```

#### WebSocket Subscription

```python
import websocket
import json

def on_message(ws, message):
    data = json.loads(message)
    print(data)

ws = websocket.WebSocketApp(
    "wss://api.predifi.com/api/v1/ws?address=GD...",
    header=["Authorization: Bearer YOUR_JWT_TOKEN"],
    on_message=on_message
)
ws.run_forever()
```

---

## Error Responses

### Standard Error Format

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable error message"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `RATE_LIMIT_EXCEEDED` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Internal server error |
| `DATABASE_UNAVAILABLE` | 503 | Database unavailable |

### Example Error Responses

#### Rate Limit Exceeded

```json
{
  "error": "RATE_LIMIT_EXCEEDED",
  "message": "Rate limit exceeded. Retry after 30 seconds."
}
```

**Headers**:
```http
HTTP/1.1 429 Too Many Requests
Retry-After: 30
```

#### Not Found

```json
{
  "error": "NOT_FOUND",
  "message": "Pool with ID 999 not found"
}
```

**Headers**:
```http
HTTP/1.1 404 Not Found
```

#### Unauthorized

```json
{
  "error": "UNAUTHORIZED",
  "message": "Missing or invalid authorization token"
}
```

**Headers**:
```http
HTTP/1.1 401 Unauthorized
```

---

## OpenAPI Specification

The complete OpenAPI specification is available at:

```
GET /api-docs/openapi.json
```

### Generate Client SDKs

#### TypeScript (using `openapi-typescript`)

```bash
npx openapi-typescript \
  https://api.predifi.com/api-docs/openapi.json \
  -o src/client/generated.ts
```

#### Python (using `openapi-python-client`)

```bash
openapi-python-client \
  --url https://api.predifi.com/api-docs/openapi.json \
  --output-path ./client
```

#### Postman Collection

Import the OpenAPI spec into Postman to generate an API collection automatically.

---

## Documentation Sources

- **OpenAPI Source**: `backend/src/openapi.rs`
- **Backend Implementation**: `backend/src/server.rs`
- **WebSocket Implementation**: `backend/src/ws.rs`
- **Constants**: `backend/src/constants.rs`

---

## Support

- **API Documentation**: `/api-docs/openapi.json`
- **Swagger UI**: Available in development builds
- **Project Repository**: [predifi/predifi](https://github.com/Web3Novalabs/predifi)
- **Issues**: GitHub Issues

---

**Last Updated**: 2026-08-28
