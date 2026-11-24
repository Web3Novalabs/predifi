# PrediFi Project Structure

This document provides an annotated overview of the repository directory tree for PrediFi up to two directory levels deep.

```
predifi/
├── backend/                  # Rust/Axum backend API service and indexer
│   ├── .sqlx/                # Cached SQL query metadata for compile-time query verification
│   ├── grafana/              # Dashboards and monitoring configurations for backend service
│   ├── migrations/           # PostgreSQL database schema migrations managed by SQLx
│   ├── src/                  # Axum application routes, state management, database handlers, and background workers
│   └── tests/                # Integration and end-to-end API test suites
├── contract/                 # Soroban smart contracts written in Rust
│   ├── contracts/            # Modular smart contract workspace crates
│   │   ├── access-control/   # Role-based access control and admin authorization logic
│   │   ├── predifi-contract/ # Core prediction market protocol logic (pools, predictions, resolutions, claims)
│   │   └── predifi-errors/   # Standardized error definitions and error code mappings
│   ├── scripts/              # Build, optimization, WASM size check, and deployment shell scripts
│   └── target/               # Compiled Wasm artifacts and build caches
├── frontend/                 # Next.js / TypeScript Web3 web application
│   ├── __tests__/            # Jest and React Testing Library frontend unit and component tests
│   ├── app/                  # Next.js App Router pages, layouts, and API route handlers
│   ├── components/           # Reusable UI components (buttons, market cards, navigation, forms)
│   ├── lib/                  # Web3 providers, Soroban SDK helpers, utilities, and API clients
│   ├── public/               # Static assets (images, icons, fonts, manifest files)
│   ├── scripts/              # Build utilities, lighthouse checks, and code generators
│   └── types/                # TypeScript type definitions and interfaces
├── docs/                     # Technical documentation, architectural specs, deployment guides, and references
├── docker/                   # Docker deployment configs and infrastructure service definitions
│   ├── grafana/              # Containerized Grafana service setup and default dashboards
│   ├── loki/                 # Containerized Loki log aggregation configuration
│   └── promtail/             # Promtail log shipping agent configuration
└── terraform/                # Infrastructure as Code (IaC) for cloud deployment
    ├── environments/         # Environment-specific configuration roots (e.g. production)
    └── modules/              # Reusable terraform modules (compute, database, monitoring, networking)
```

---

## Directory Descriptions

### `backend/`
The `backend/` service is built in Rust using the Axum web framework and SQLx. It provides REST API endpoints, indexes Soroban contract events, manages database persistence, handles user referrals, and serves telemetry metrics.
- **`.sqlx/`**: Offline SQL query data used by SQLx for compile-time query checking.
- **`grafana/`**: Metrics dashboards and alert definitions for backend operations.
- **`migrations/`**: SQL migration scripts defining database tables, indexes, and constraints.
- **`src/`**: Primary application source code (routes, database modules, config, middleware).
- **`tests/`**: Integration test suite verifying backend API endpoints and DB operations.

### `contract/`
The `contract/` workspace contains the Soroban smart contracts compiled to Wasm (`wasm32-unknown-unknown`) target.
- **`contracts/access-control/`**: Crate implementing permission controls and admin roles for the protocol.
- **`contracts/predifi-contract/`**: Main contract crate implementing prediction pool initialization, placing predictions, oracle resolution, fee distribution, and reward claims.
- **`contracts/predifi-errors/`**: Shared crate defining gap-based protocol error codes and helper functions.
- **`scripts/`**: Shell scripts for building, size checking (`wasm_size_check.sh`), and deploying (`deploy.sh`).

### `frontend/`
The `frontend/` application is built using Next.js (App Router), React, TypeScript, and Tailwind CSS.
- **`__tests__/`**: Unit and UI tests executed via Jest.
- **`app/`**: Application routes, pages, modals, and layouts using Next.js App Router conventions.
- **`components/`**: Modular UI components (MarketCard, WalletConnect, Navigation, Modals).
- **`lib/`**: Client-side logic including wallet interactions, contract SDK calls, and state hooks.
- **`public/`**: Static assets such as logos, icons, and web manifest files.
- **`scripts/`**: Development scripts and code-generation tools.
- **`types/`**: Shared TypeScript types for API contracts, market models, and wallet state.

### `docs/`
Contains comprehensive project documentation including architectural diagrams, error handling specifications, oracle integrations, accessibility guidelines, security audits, and quickstart guides.

### `docker/`
Contains container orchestrations and log/metrics monitoring stacks.
- **`grafana/`**: Dashboard definitions and Grafana provisioning scripts.
- **`loki/`**: Loki log collection service configuration.
- **`promtail/`**: Log scraper agent sending logs to Loki.

### `terraform/`
Contains infrastructure declarations written in HashiCorp Terraform for provisioning cloud resources.
- **`environments/`**: Environment deployment targets (such as `production/`).
- **`modules/`**: Modular Terraform code for compute (EC2/K8s), database (PostgreSQL), caching (Redis), DNS, and SSL.
