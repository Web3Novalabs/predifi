# PrediFi Documentation

This directory contains comprehensive documentation for the PrediFi prediction market platform.

---

## Documentation Index

| Document | Description |
|----------|-------------|
| **[API_REFERENCE.md](./API_REFERENCE.md)** | Complete API reference with OpenAPI specification, authentication, rate limiting, and WebSocket subscriptions |
| **[OPENAPI_CLIENT_GENERATION.md](./OPENAPI_CLIENT_GENERATION.md)** | Guide for generating client SDKs in TypeScript, Python, and Rust |
| **[SMART_CONTRACT_DEPLOYMENT_GUIDE.md](./SMART_CONTRACT_DEPLOYMENT_GUIDE.md)** | Step-by-step guide for deploying PrediFi smart contracts to Stellar testnet and mainnet |
| **[quickstart.md](./quickstart.md)** | Quick start guide for developers |
| **[architecture_overview.md](./ARCHITECTURE_OVERVIEW.md)** | System architecture and component overview |
| **[error_handling_reference.md](./ERROR_HANDLING_REFERENCE.md)** | Comprehensive error codes and troubleshooting |
| **[troubleshooting.md](./troubleshooting.md)** | Common issues and solutions |
| **[prediction_lifecycle.md](./PREDICTION_LIFECYCLE.md)** | Prediction market lifecycle documentation |
| **[oracles.md](./ORACLES.md)** | Oracle integration documentation |
| **[whitelist_events.md](./WHITELIST_EVENTS.md)** | Token whitelist events documentation |

---

## Quick Links

### For Developers

1. **Start Here**: [Quickstart Guide](./quickstart.md)
2. **API Documentation**: [API Reference](./API_REFERENCE.md)
3. **Smart Contract Deployment**: [Deployment Guide](./SMART_CONTRACT_DEPLOYMENT_GUIDE.md)
4. **Client SDKs**: [OpenAPI Client Generation](./OPENAPI_CLIENT_GENERATION.md)

### For Operators

1. **Smart Contract Deployment**: [Deployment Guide](./SMART_CONTRACT_DEPLOYMENT_GUIDE.md)
2. **System Architecture**: [Architecture Overview](./ARCHITECTURE_OVERVIEW.md)
3. **Error Handling**: [Error Reference](./ERROR_HANDLING_REFERENCE.md)

### For Integrators

1. **API Reference**: [API Reference](./API_REFERENCE.md)
2. **WebSocket Subscriptions**: [WebSocket Documentation](./API_REFERENCE.md#websocket-subscriptions)
3. **Rate Limiting**: [Rate Limiting](./API_REFERENCE.md#rate-limiting)

---

## Documentation Structure

```
docs/
├── README.md                          # This file
├── API_REFERENCE.md                   # Complete API reference
├── OPENAPI_CLIENT_GENERATION.md       # SDK generation guide
├── SMART_CONTRACT_DEPLOYMENT_GUIDE.md # Contract deployment guide
├── quickstart.md                      # Quick start guide
├── ARCHITECTURE_OVERVIEW.md           # System architecture
├── ERROR_HANDLING_REFERENCE.md        # Error codes
├── troubleshooting.md                 # Common issues
├── prediction_lifecycle.md            # Prediction lifecycle
├── oracles.md                         # Oracle integration
├── whitelist_events.md                # Whitelist events
└── health-check-endpoint.md           # Health check details
```

---

## Getting Started

### Prerequisites

- Rust and Cargo (latest stable)
- Node.js and npm (latest LTS)
- Git

### Local Development

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd predifi
   ```

2. **Build the backend**
   ```bash
   cd backend
   cargo build --release
   ```

3. **Start the development server**
   ```bash
   cargo run --release
   ```

4. **Access the API documentation**
   - Swagger UI: `http://localhost:8000/api-docs/`
   - OpenAPI JSON: `http://localhost:8000/api-docs/openapi.json`

### Running Tests

```bash
# Backend tests
cd backend
cargo test

# Frontend tests
cd frontend
npm test
```

---

## API Endpoints Overview

### Public Endpoints (No Auth Required)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/v1/pools` | GET | List prediction markets |
| `/api/v1/pools/{pool_id}` | GET | Get pool details |
| `/api/v1/stats` | GET | Protocol statistics |
| `/api/v1/fees` | GET | Fee configuration |
| `/api/v1/leaderboard` | GET | User rankings |

### Protected Endpoints (JWT Required)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/users/{address}/history` | GET | User prediction history |
| `/api/v1/users/{address}/predictions` | GET | Enhanced user predictions |
| `/api/v1/referrals/{address}` | GET | Referral earnings |
| `/api/v1/indexer/*` | POST | Event ingestion (internal) |

### WebSocket Endpoints

| Endpoint | Description |
|----------|-------------|
| `/api/v1/ws` | Live event subscription |

---

## Authentication

All protected endpoints require a valid JWT access token:

```
Authorization: Bearer <jwt_access_token>
```

### Token Structure

```json
{
  "sub": "GD... wallet address",
  "type": "access",
  "exp": 1234567890,
  "iat": 1234567890,
  "key_version": 0
}
```

---

## Rate Limiting

PrediFi implements tiered rate limiting:

| Tier | Burst | Period | Sustained Rate |
|------|-------|--------|----------------|
| Light | 120 | 60s | 2 req/s |
| Read | 60 | 60s | 1 req/s |
| User | 30 | 60s | 0.5 req/s |
| Write | 20 | 60s | 0.33 req/s |
| Token | 10 | 60s | 0.16 req/s |
| WebSocket | 10 | 10s | 1 msg/s |

---

## WebSocket Subscriptions

Subscribe to live events:

```
wss://api.predifi.com/api/v1/ws?address=GD...
```

### Event Types

- `prediction_placed` - New prediction
- `pool_created` - New prediction market
- `pool_resolved` - Market resolution

---

## Smart Contract Deployment

See [SMART_CONTRACT_DEPLOYMENT_GUIDE.md](./SMART_CONTRACT_DEPLOYMENT_GUIDE.md) for:

- Wallet setup
- Network configuration
- Contract compilation
- Testnet and mainnet deployment
- Initialization parameters
- Token whitelisting
- Oracle registration
- Post-deployment verification

---

## Support

- **Documentation**: Check the docs directory
- **API Issues**: GitHub Issues
- **Discord**:predifi Discord server
- **Email**: dev@predifi.com

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Update documentation as needed
4. Submit a pull request

---

## License

MIT License - See LICENSE file for details.

---

**Last Updated**: 2026-08-28
