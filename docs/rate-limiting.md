# Rate Limiting Reference

The PrediFi backend implements granular, IP-based rate limiting across all API endpoints and WebSocket connections to protect the infrastructure against denial-of-service (DoS) attacks, brute-force attempts, and resource exhaustion.

---

## Rate-Limit Tiers

Rate limits are configured in `backend/src/constants.rs` and enforced using [tower-governor](https://github.com/caspervonb/tower-governor). Each route group is assigned to a tier calibrated for its expected traffic profile and computational cost.

| Tier | Burst Size (Tokens) | Replenishment Window | Sustained Rate | Protected Endpoints | Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Light** | `120` | `60 s` | ~2.0 req/s | `GET /`<br>`GET /health`<br>`GET /fees`<br>`GET /prices` | Cheap, stateless, and polling-friendly endpoints |
| **Read** | `60` | `60 s` | ~1.0 req/s | `GET /pools`<br>`GET /pools/:id`<br>`GET /pools/:id/leaderboard`<br>`GET /stats`<br>`GET /leaderboard`<br>`GET /tags`<br>`GET /referrals/{address}`<br>`GET /referrals/:address/estimate`<br>`GET /markets/:id/predictions` | Public database-backed read and query endpoints |
| **User** | `30` | `60 s` | ~0.5 req/s | `GET /users/{address}/history`<br>`GET /users/{address}/predictions`<br>`GET /users/{address}/profile`<br>`GET /users/:address/referrals`<br>`GET /users/{address}/interests`<br>`PUT /users/{address}/interests`<br>`GET /notifications/{address}`<br>`POST /notifications/{address}/read` | Per-user history, user profiles, interests, and notifications |
| **Write** | `20` | `60 s` | ~0.33 req/s | `POST /indexer/pool-created`<br>`POST /indexer/prediction-placed`<br>`POST /indexer/claim`<br>`PATCH /pools/:id/tags` | Indexer ingest operations and state-mutation endpoints |
| **Token** | `10` | `60 s` | ~0.17 req/s | `POST /auth/refresh`<br>`GET /ws` (handshake) | Authentication token rotation and WebSocket handshake; prevents token brute-forcing |
| **WebSocket** | `10` | `10 s` | ~1.0 msg/s | Inbound WebSocket messages | Per-connection message limit preventing client message flooding |
| **Default / Fallback** | `100` | `900 s` (15 min) | ~1 token / 9 s | Global fallback routes | General protection for unclassified endpoints |

---

## Token-Bucket Algorithm

Rate limiting in PrediFi operates on the **token-bucket algorithm**:

```
           Token Bucket
     ┌──────────────────────┐
     │  ●   ●   ●   ●   ●   │  <-- Refilled at constant rate (Period / Burst)
     │  ●   ●   ●   ●   ●   │  <-- Capacity = Burst Size
     └──────────┬───────────┘
                │
         Incoming Request
                │
                ▼
        [ Token Available? ]
           /          \
        Yes            No
        /                \
   [ Consume 1 ]      [ HTTP 429 ]
   [ Process ]       [ Rate Limit Exceeded ]
```

1. **Bucket Capacity (Burst Size)**: Each client IP has a virtual bucket with a maximum capacity equal to the tier's burst size (e.g. 60 tokens for the `Read` tier).
2. **Token Consumption**: Every request immediately consumes **1 token**. If the bucket is non-empty, the request is allowed through.
3. **Continuous Replenishment**: Tokens are replenished smoothly and continuously at a rate calculated as:
   $$\text{Replenishment Interval} = \frac{\text{Period}}{\text{Burst}}$$
   *Example:* For the `Read` tier (60 requests per 60 seconds), 1 token is added back every 1 second.
4. **Bucket Depletion**: When the bucket is empty (0 tokens remaining), subsequent requests are rejected immediately with `HTTP 429 Too Many Requests` until tokens replenish.

---

## Client IP & Proxy Handling

The rate limiter identifies clients using `SmartIpKeyExtractor`, which resolves client IPs in the following precedence order:
1. `X-Forwarded-For` header (first untrusted hop when behind a reverse proxy/load balancer).
2. `X-Real-IP` header.
3. Direct TCP socket address of the incoming connection.

---

## Handling HTTP 429 (Too Many Requests)

When a client exceeds the allowable rate limit, the server responds with:
- **HTTP Status Code**: `429 Too Many Requests`
- **Content-Type**: `application/json`

### Error Response Body
The response body conforms to the standard PrediFi error envelope:

```json
{
  "status": "error",
  "error": "Too many requests"
}
```

### Recommended Client Strategies

To handle rate limits cleanly without degrading user experience, API consumers and frontends should implement the following patterns:

#### 1. Exponential Backoff with Jitter
When encountering an `HTTP 429`, retry the request with exponential backoff plus random jitter to avoid thundering herd problems:

```typescript
async function fetchWithRetry(url: string, options: RequestInit, retries = 3, backoff = 1000): Promise<Response> {
  for (let attempt = 0; attempt < retries; attempt++) {
    const res = await fetch(url, options);
    if (res.status !== 429) {
      return res;
    }
    
    // Calculate exponential backoff with jitter
    const jitter = Math.random() * 200;
    const delay = backoff * Math.pow(2, attempt) + jitter;
    console.warn(`Rate limited (429). Retrying in ${Math.round(delay)}ms...`);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
  throw new Error("Rate limit exceeded after maximum retries");
}
```

#### 2. Local Request Throttling & Debouncing
- **Search and filter inputs**: Debounce user keystrokes (e.g. 300ms) before dispatching requests to `/pools` or `/tags`.
- **Polling loops**: For dashboards and price updates, use polling intervals that align with the tier limits (e.g. poll `/prices` no faster than every 1–2 seconds).

#### 3. WebSocket Batching
- Batch high-frequency events into single payloads rather than sending individual messages per action.

---

## Environment & Testing Behavior

- **Production / Staging**: Full rate limiting is active across all tiers.
- **Unit & Integration Tests (`#[cfg(test)]`)**: Rate limiting middleware is a no-op so parallel test suites do not cross-contaminate token buckets.
