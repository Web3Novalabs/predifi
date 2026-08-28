# Contributor Frequently Asked Questions (FAQ)

This FAQ answers common questions asked by new contributors setting up, developing, testing, and troubleshooting the PrediFi project.

---

## 1. Which directory do I work in?

The PrediFi repository is organized into distinct sub-projects based on tech stack and layer:

| Directory | Layer / Tech Stack | What Lives Here |
| :--- | :--- | :--- |
| `frontend/` | Next.js 15, React 19, TailwindCSS, TypeScript | Web application UI, wallet connection hooks, charts, and client-side market browsing |
| `backend/` | Rust (Axum, Tokio, SQLx, Redis) | REST API server, database indexer, WebSocket real-time event bus, price caching, and authentication |
| `contract/` | Rust (Soroban SDK) | Smart contract workspace containing `predifi-contract` (pools & bets), `access-control`, and `predifi-errors` |
| `docker/` | Docker, Loki, Promtail, Grafana | Container configs, log scraping rules, and monitoring dashboard definitions |
| `terraform/` | Terraform (AWS) | Infrastructure-as-code modules for compute (ASG), PostgreSQL (RDS), Redis (ElastiCache), ALB, DNS, and SSL |
| `docs/` | Markdown | Technical documentation, architecture guides, tutorials, and API specifications |

---

## 2. Do I need a Stellar testnet account to run the frontend?

**No** for general UI and component development. The frontend can run in mock/read-only mode without a connected wallet to inspect layouts, pages, and components.

**Yes** if you want to test transaction signing or place live testnet predictions:
1. Install a Stellar wallet browser extension like [Freighter](https://freighter.app/).
2. Switch the network in Freighter settings to **Testnet**.
3. Create or import an account and fund it with free testnet XLM via the [Stellar Laboratory Friendbot](https://laboratory.stellar.org/#account-creator?network=test).

---

## 3. How do I run just the frontend tests?

You can run the frontend test suite using `pnpm`:

```bash
cd frontend && pnpm test
```

### Useful test commands
- **Run a single test file**: `cd frontend && pnpm test -- src/components/MarketCard.test.tsx`
- **Run in watch mode**: `cd frontend && pnpm test -- --watch`
- **Generate coverage report**: `cd frontend && pnpm test -- --coverage`

---

## 4. Why does the backend need PostgreSQL and Redis?

The backend uses both data stores for distinct, complementary purposes:

- **PostgreSQL**:
  - Acts as the primary relational database for indexed on-chain events (pool creation, predictions placed, rewards claimed).
  - Stores off-chain metadata, user profiles, market tags, referral relationships, and protocol historical metrics.
  - Managed via `sqlx` migrations located in `backend/migrations/`.
- **Redis**:
  - Serves as a high-speed cache for hot, frequently requested data (e.g. active pool listings, oracle price feeds).
  - Backs WebSocket subscriptions and event broadcast channels across backend instances.
  - Stores transient user sessions and rate-limiting buckets.

---

## 5. What do I do if `cargo test` is slow?

Rust test compilation can take time, especially on cold builds. Here are recommended strategies to speed up your local workflow:

1. **Test only the specific crate or function**:
   ```bash
   # Instead of running the whole workspace, run only the unit test you are working on:
   cd backend && cargo test validated_types
   cd contract && cargo test test_odds_calculation
   ```
2. **Run fast syntax/type checks before testing**:
   ```bash
   cd backend && cargo check
   ```
3. **Use `sccache` for compiler artifact caching**:
   ```bash
   cargo install sccache
   export RUSTC_WRAPPER=sccache
   ```
4. **Skip containerized integration tests during quick iterations**:
   - `cargo test` runs fast in-memory unit tests.
   - Only add `--features integration-tests` when you are ready to test real database roundtrips.

---

## 6. How do I spin up the entire local development stack?

The repository includes a top-level `docker-compose.yml` that provisions all backend dependencies, local databases, and development servers:

```bash
docker compose up
```

This starts:
- **Frontend** (<http://localhost:3001>)
- **Backend API** (<http://localhost:3000>)
- **PostgreSQL** (`localhost:5432` - `postgres`/`postgres`/`predifi`)
- **Redis** (`localhost:6379`)
- **Stellar Local Network Node** (<http://localhost:8000>)
- **Database Seeder (`db-seed`)** (automatically applies migrations and loads sample data)

To shut down and wipe volumes:
```bash
docker compose down -v
```

---

## 7. How do I re-run migrations and reset seed data?

The `predifi-seed` binary applies all SQL migrations and seeds deterministic sample pools and users:

```bash
docker compose run --rm db-seed cargo run --bin predifi-seed -- --fresh
```

The `--fresh` flag drops existing tables and reapplies the schema from scratch.

---

## 8. How do I test and build the Soroban smart contracts?

From the `contract/` workspace root:

```bash
# Run unit tests across all contracts
cd contract && cargo test

# Run the automated workspace test suite
bash contract/scripts/test_all.sh

# Build the WASM contract binaries
soroban contract build
```

---

## 9. How do I export the OpenAPI spec and sync frontend API types?

The backend generates an OpenAPI v3 specification from code annotations using `utoipa`:

```bash
# 1. Export OpenAPI spec from backend
cd backend && cargo run --bin predifi-openapi

# 2. Generate TypeScript types in the frontend
cd frontend && pnpm generate:api
```

---

## 10. How is rate limiting handled in local development?

Rate limits are configured in `backend/src/constants.rs` across 5 primary tiers (Light, Read, User, Write, Token).

- In development/production servers, rate limits are active per client IP.
- In unit tests (`#[cfg(test)]`), the rate limiter middleware is automatically disabled to prevent tests from interfering with each other's token buckets.

For full tier specifications and client retry advice, see [`docs/rate-limiting.md`](./rate-limiting.md).

---

## 11. How do I view logs and metrics in Grafana?

To start the optional logging and observability stack:

```bash
docker compose --profile logging up
```

- Open Grafana at <http://localhost:3002> (`admin` / `admin`).
- The **PrediFi — Logs & Error Trends** dashboard is pre-provisioned to query Loki logs collected by Promtail from all running containers.
- Prometheus metrics are available at <http://localhost:9090>.

---

## 12. Where should I ask questions or report bugs?

- **GitHub Issues**: For bug reports, feature requests, or task assignments, open an issue on the [PrediFi GitHub repository](https://github.com/Web3Novalabs/predifi/issues).
- **Telegram Group**: For real-time discussion and contributor help, join the [PrediFi Community](https://t.me/predifi_onchain_build/1).
