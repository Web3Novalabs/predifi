# OpenAPI Client Generation Guide

This document provides instructions for generating SDKs from the PrediFi OpenAPI specification.

---

## Available OpenAPI Specification

The complete OpenAPI 3.0 specification is available at:

```
https://api.predifi.com/api-docs/openapi.json
```

For local development:

```
http://localhost:8000/api-docs/openapi.json
```

---

## Generated SDKs

We maintain client SDKs for the following languages:

| Language | Repository | Status |
|----------|-----------|--------|
| TypeScript | `frontend/lib/api/generated.ts` | ✅ Maintained |
| Python | `tools/python-client/` | 🔄 In progress |
| Rust | `tools/rust-client/` | ⏳ Planned |

---

## Generating Your Own SDK

### TypeScript/JavaScript

#### Using `openapi-typescript`

```bash
npx openapi-typescript \
  https://api.predifi.com/api-docs/openapi.json \
  -o src/client/generated.ts
```

**Options**:
- `--client axios` - Generate axios-based client
- `--client fetch` - Generate fetch-based client
- `--client superagent` - Generate superagent-based client

#### Using `openapi-client-axios`

```bash
npx openapi-client-axios \
  https://api.predifi.com/api-docs/openapi.json \
  --output src/client/generated.ts
```

#### Manual TypeScript Client Example

```typescript
import axios, { AxiosInstance } from 'axios';

export class PrediFiClient {
  private readonly client: AxiosInstance;

  constructor(baseURL: string, apiKey?: string) {
    this.client = axios.create({
      baseURL,
      headers: apiKey ? { Authorization: `Bearer ${apiKey}` } : {},
    });
  }

  // Health Check
  async health(): Promise<HealthResponse> {
    return this.client.get('/health').then(r => r.data);
  }

  // Get Pools
  async getPools(params?: {
    sortBy?: 'popular' | 'ending_soon' | 'new';
    category?: string;
    status?: 'active' | 'closed' | 'settled';
    limit?: number;
    offset?: number;
  }): Promise<PoolListResponse> {
    return this.client.get('/api/v1/pools', { params }).then(r => r.data);
  }

  // Get Pool by ID
  async getPoolById(poolId: number): Promise<PoolWithOddsDoc> {
    return this.client.get(`/api/v1/pools/${poolId}`).then(r => r.data);
  }

  // Get Protocol Stats
  async getStats(): Promise<ProtocolStatsDoc> {
    return this.client.get('/api/v1/stats').then(r => r.data);
  }

  // Get Fees
  async getFees(): Promise<FeeInfoDoc> {
    return this.client.get('/api/v1/fees').then(r => r.data);
  }

  // Get Leaderboard
  async getLeaderboard(params?: {
    rankBy?: 'volume' | 'winnings';
    limit?: number;
    offset?: number;
  }): Promise<LeaderboardResponse> {
    return this.client.get('/api/v1/leaderboard', { params }).then(r => r.data);
  }

  // Get User History
  async getUserHistory(
    address: string,
    params?: { limit?: number; offset?: number }
  ): Promise<PredictionHistoryResponse> {
    return this.client.get(`/api/v1/users/${address}/history`, { params }).then(r => r.data);
  }

  // Get Market Predictions
  async getMarketPredictions(
    marketId: number,
    params?: { after?: number; limit?: number }
  ): Promise<MarketPredictionsResponse> {
    return this.client.get(`/api/v1/markets/${marketId}/predictions`, { params }).then(r => r.data);
  }
}
```

### Python

#### Using `openapi-python-client`

```bash
# Install
pip install openapi-python-client

# Generate client
openapi-python-client \
  --url https://api.predifi.com/api-docs/openapi.json \
  --output-path ./predifi_client

# Install generated client
cd predifi_client
pip install .

# Use in your code
from predifi_client import PrediFiClient

client = PrediFiClient(base_url="https://api.predifi.com/v1", api_key="YOUR_API_KEY")

# Get pools
pools = client.get_pools(sort_by="popular", category="Crypto")

# Get health
health = client.health()
```

#### Using `requests` with manual client

```python
import requests
from typing import Optional

class PrediFiClient:
    def __init__(self, base_url: str, api_key: Optional[str] = None):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        if api_key:
            self.session.headers['Authorization'] = f'Bearer {api_key}'

    def _get(self, path: str, params: Optional[dict] = None) -> dict:
        response = self.session.get(f"{self.base_url}{path}", params=params)
        response.raise_for_status()
        return response.json()

    def health(self) -> dict:
        return self._get('/health')

    def get_pools(self, sort_by: Optional[str] = None, category: Optional[str] = None) -> dict:
        params = {'sort_by': sort_by, 'category': category}
        return self._get('/api/v1/pools', params)

    def get_pool_by_id(self, pool_id: int) -> dict:
        return self._get(f'/api/v1/pools/{pool_id}')

    def get_stats(self) -> dict:
        return self._get('/api/v1/stats')

    def get_leaderboard(self, rank_by: Optional[str] = None) -> dict:
        params = {'rank_by': rank_by}
        return self._get('/api/v1/leaderboard', params)
```

### Rust

#### Using `utoipa` (Rust OpenAPI)

```rust
use utoipa::OpenApi;
use serde::{Deserialize, Serialize};

// The OpenAPI spec is defined in backend/src/openapi.rs
// and can be exported with: cargo run --bin predifi-openapi

// For client generation, consider using:
// - reqwest for HTTP
// - serde for serialization
```

#### Manual Rust Client

```rust
use reqwest::{Client, Response};
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

pub struct PrediFiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl PrediFiClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    pub async fn health(&self) -> Result<HealthResponse, Box<dyn Error>> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    pub async fn get_pools(&self, sort_by: Option<&str>) -> Result<serde_json::Value, Box<dyn Error>> {
        let mut url = format!("{}/api/v1/pools", self.base_url);
        if let Some(sort_by) = sort_by {
            url.push_str(&format!("?sort_by={}", sort_by));
        }
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }
}
```

---

## Using the OpenAPI JSON Directly

### Postman

1. Open Postman
2. Click **Import**
3. Drag/drop or select `openapi.json`
4. Postman generates collections automatically

### Swagger UI

1. Navigate to `https://editor.swagger.io/`
2. Click **File** → **Import URL**
3. Enter: `https://api.predifi.com/api-docs/openapi.json`
4. View and test the API

### Insomnia

1. Open Insomnia
2. Click **Import** → **Import Data**
3. Select `openapi.json`
4. Insomnia generates requests

### curl

Generate curl commands from the spec:

```bash
# Export OpenAPI spec
curl https://api.predifi.com/api-docs/openapi.json > openapi.json

# Convert to curl commands using tools like:
# - openapi2curl
# - swagger2circus
```

---

## TypeScript Client Integration

### Example: Using in React

```typescript
import { useState, useEffect } from 'react';
import { PrediFiClient } from './client/generated';

export function usePools() {
  const [pools, setPools] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    const client = new PrediFiClient('https://api.predifi.com/v1');
    
    client.getPools({ sortBy: 'popular' })
      .then(data => {
        setPools(data.pools);
        setLoading(false);
      })
      .catch(err => {
        setError(err);
        setLoading(false);
      });
  }, []);

  return { pools, loading, error };
}
```

### Example: WebSocket with React

```typescript
import { useState, useEffect, useCallback } from 'react';

export function useWebSocket(address: string) {
  const [events, setEvents] = useState([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const token = localStorage.getItem('jwt_token');
    const ws = new WebSocket(
      `wss://api.predifi.com/api/v1/ws?address=${address}`
    );

    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setEvents(prev => [data, ...prev]);
    };

    return () => {
      ws.close();
    };
  }, [address]);

  return { events, connected };
}
```

---

## Validation and Testing

### Validate OpenAPI Spec

```bash
# Using Spectral
npm install -g @stoplight/spectral
spectral lint https://api.predifi.com/api-docs/openapi.json

# Using openapi-spec-validator
pip install openapi-spec-validator
openapi-spec-validator https://api.predifi.com/api-docs/openapi.json
```

### Test Generated Client

```bash
# Run generated tests
npm test
# or
yarn test
```

---

## API Documentation Links

- **OpenAPI JSON**: `/api-docs/openapi.json`
- **Swagger UI**: Available in development
- **Postman Collection**: Available upon request
- **TypeScript Definitions**: `frontend/lib/api/generated.ts`

---

## Troubleshooting

### Common Issues

#### 1. TypeScript generation fails

**Solution**: Check that the OpenAPI spec is valid JSON and properly formatted.

#### 2. Python client imports fail

**Solution**: Ensure all dependencies are installed:
```bash
pip install requests typing-extensions
```

#### 3. Rate limits are too restrictive

**Solution**: Implement exponential backoff in your client:
```javascript
async function retryRequest(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (err) {
      if (err.response?.status === 429) {
        const retryAfter = parseInt(err.response.headers['retry-after'], 10);
        await new Promise(r => setTimeout(r, retryAfter * 1000));
      } else {
        throw err;
      }
    }
  }
}
```

---

## Contributing

When adding new API endpoints:

1. Update `backend/src/openapi.rs` with the endpoint documentation
2. Add schema definitions in `schemas`
3. Add path definitions in `paths`
4. Update the TypeScript client (`frontend/lib/api/`)
5. Test the generated client

---

## Support

- **API Documentation**: `/api-docs/openapi.json`
- **Project Repository**: predifi/predifi
- **Issues**: GitHub Issues
- **Discord**:predifi Discord server

---

**Last Updated**: 2026-08-28
