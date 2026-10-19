# PrediFi Architecture Overview

**Version**: 1.0.0  
**Scope**: Smart Contract Layer, Backend Service, Frontend State Management, and End-to-End Component Interactions.  
**Platform**: Stellar / Soroban (Protocol 23, Soroban SDK 23.5.3)  

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Technology Stack](#technology-stack)
3. [System Component Architecture](#system-component-architecture)
4. [Smart Contract Architecture & Interactions](#smart-contract-architecture--interactions)
   - [Contract Interaction Diagram](#contract-interaction-diagram)
   - [Contract Modules & Responsibilities](#contract-modules--responsibilities)
   - [Ledger Storage Architecture](#ledger-storage-architecture)
5. [Backend API Flow](#backend-api-flow)
   - [Request Lifecycle Sequence Diagram](#request-lifecycle-sequence-diagram)
   - [Backend Subsystems & State Structure](#backend-subsystems--state-structure)
   - [Caching Hierarchy & Invalidation Strategy](#caching-hierarchy--invalidation-strategy)
6. [Frontend State Management](#frontend-state-management)
   - [State Management Architecture Diagram](#state-management-architecture-diagram)
   - [Client-Side Data Flow & Cache Strategy](#client-side-data-flow--cache-strategy)
   - [Session & Persistence Fallback Layer](#session--persistence-fallback-layer)
   - [Transaction Signing & Simulation Lifecycle](#transaction-signing--simulation-lifecycle)
7. [End-to-End Data Flow](#end-to-end-data-flow)
   - [Pool Creation Flow](#pool-creation-flow)
   - [Prediction Staking & Event Indexing Flow](#prediction-staking--event-indexing-flow)
   - [Oracle Price Updates & Automated Resolution](#oracle-price-updates--automated-resolution)
   - [Payout Settlement & Fee Routing Flow](#payout-settlement--fee-routing-flow)
8. [Prediction Market Lifecycle State Machine](#prediction-market-lifecycle-state-machine)
9. [Security Invariants & Reentrancy Guards](#security-invariants--reentrancy-guards)
10. [Monitoring, Telemetry & Observability](#monitoring-telemetry--observability)

---

## Executive Summary

PrediFi is a decentralized, non-custodial prediction market protocol deployed on the Stellar network using Soroban smart contracts. It enables users to create binary and categorical markets, stake positions using whitelisted Stellar tokens (including native XLM and stablecoins), and claim provably fair payouts upon outcome resolution.

The architecture combines on-chain financial guarantees and immutable settlement with a high-throughput, low-latency off-chain indexing and caching layer to power modern, responsive web experiences.

---

## Technology Stack

| Layer | Component | Technology | Purpose |
|-------|-----------|------------|---------|
| **Blockchain** | Network | Stellar Mainnet / Testnet (Protocol 23) | Decentralized settlement and consensus. |
| **Smart Contracts** | Runtime | Soroban WASM (`no_std` Rust, SDK 23.5.3) | Execution of pools, staking, payouts, and access control. |
| **Backend** | Web Framework | Rust, Axum 0.7, Tokio | High-performance asynchronous HTTP and WebSocket server. |
| **Persistence** | Relational DB | PostgreSQL 16 (SQLx) | Indexed relational storage for fast querying of pools and history. |
| **Caching** | Caches | Redis 7 + In-Memory `PoolCache` & `PriceCache` | Hot read caching, rate limiting tokens, and price feeds. |
| **Frontend** | Application | Next.js 15 (React 19, TypeScript, Tailwind) | User-facing responsive web application and wallet interface. |
| **Wallet** | Signer | Freighter Wallet & Stellar SDK | Client-side transaction building, simulation, and cryptographic signing. |
| **Oracles** | Data Feeds | Pyth Network Adapter & Whitelisted Oracles | Real-world price and outcome feeds for automated market settlement. |
| **Telemetry** | Observability | Prometheus & OpenTelemetry (Tracing) | Metrics scraping, request latency profiling, and audit logging. |

---

## System Component Architecture

The diagram below details the operational boundaries and network protocols connecting clients, backend services, external oracles, and the Stellar ledger.

```mermaid
flowchart TB
    subgraph ClientLayer["Client Layer"]
        User["User Browser"]
        Freighter["Freighter Wallet"]
        User <--> Freighter
    end

    subgraph FrontendApp["Frontend Application (Next.js 15)"]
        UIComponents["React Server/Client Components"]
        SWRCache["SWR Data Cache"]
        PersistentStore["Cookie / LocalStorage Fallback"]
        UIComponents <--> SWRCache
        UIComponents <--> PersistentStore
    end

    subgraph BackendCluster["Backend Service (Rust / Axum)"]
        Router["Axum Router & Middleware Stack"]
        AppState["Shared AppState"]
        PoolCache["In-Memory PoolCache"]
        PriceCache["In-Memory PriceCache"]
        EventBus["WebSocket EventBus"]
        StellarWorker["Stellar Listener Worker"]
        Router --> AppState
        AppState --> PoolCache
        AppState --> PriceCache
        AppState --> EventBus
        StellarWorker --> EventBus
    end

    subgraph DataStorage["Data & Storage Layer"]
        Postgres[(PostgreSQL Database)]
        RedisCache[(Redis Cache)]
        AppState --> Postgres
        AppState --> RedisCache
        StellarWorker --> Postgres
        StellarWorker --> RedisCache
    end

    subgraph StellarNetwork["Stellar / Soroban Blockchain"]
        StellarRPC["Stellar Horizon & Soroban RPC"]
        AccessControlContract["Access Control Contract"]
        PredifiContract["PrediFi Market Contract"]
        PythOracle["Pyth Oracle / External Feeds"]
        StellarRPC --> PredifiContract
        PredifiContract <--> AccessControlContract
        PredifiContract <--> PythOracle
    end

    User <--> FrontendApp
    FrontendApp -- "REST & WebSocket (WSS)" --> Router
    FrontendApp -- "Simulate & Submit Signed TX" --> StellarRPC
    BackendCluster -- "Query Ledger & Poll Events" --> StellarRPC
```

---

## Smart Contract Architecture & Interactions

The contract layer separates administrative authorization from market mechanics to enhance upgradeability, auditability, and principle of least privilege.

### Contract Interaction Diagram

```mermaid
flowchart TD
    subgraph ExternalCallers["External Invocations"]
        AdminUser["Admin Signer"]
        OperatorUser["Operator Signer"]
        OracleFeeder["Oracle Signer"]
        MarketUser["Market Participant (Bettor/Creator)"]
    end

    subgraph AccessControl["access-control Contract"]
        RoleRegistry["Role Registry (Admin=0, Operator=1, Oracle=3)"]
        HasRoleCheck["has_role(address, role)"]
        AdminRoleGuard["require_admin(caller)"]
    end

    subgraph PredifiMarket["predifi-contract"]
        AdminModule["admin.rs (Init, Pause, Timelocked Fees, Whitelist)"]
        PoolModule["pool.rs (create_pool, resolve_pool, cancel_pool)"]
        PredictionModule["prediction.rs (place_prediction, claim_winnings)"]
        PayoutModule["payouts.rs (calculate_payout, odds, protocol fee)"]
        TreasuryModule["treasury.rs (fee collection, withdraw_treasury)"]
        ReferralModule["referral.rs (attribution, referral rewards)"]
        OracleModule["oracle.rs & price_feed.rs (Pyth condition evaluation)"]
        SafeMath["safe_math.rs (overflow-checked math)"]
        RentGuard["RentGuard (reentrancy lockout)"]
    end

    subgraph LedgerStorage["Soroban Storage Tiers"]
        InstanceStorage[("Instance Storage: Config, Whitelist, Paused")]
        PersistentStorage[("Persistent Storage: Pools, Predictions, Stakes, TTLs")]
        TempStorage[("Temporary Storage: Votes, RentGuard")]
    end

    AdminUser --> AdminModule
    AdminUser --> RoleRegistry
    OperatorUser --> PoolModule
    OracleFeeder --> OracleModule
    MarketUser --> PredictionModule
    MarketUser --> PoolModule

    PoolModule -- "Cross-Contract Role Check" --> HasRoleCheck
    OracleModule -- "Cross-Contract Role Check" --> HasRoleCheck
    AdminModule -- "Cross-Contract Role Check" --> HasRoleCheck

    PredictionModule --> RentGuard
    PredictionModule --> PayoutModule
    PredictionModule --> ReferralModule
    PredictionModule --> SafeMath
    PoolModule --> OracleModule
    PayoutModule --> TreasuryModule

    AdminModule --> InstanceStorage
    PoolModule --> PersistentStorage
    PredictionModule --> PersistentStorage
    RentGuard --> TempStorage
```

### Contract Modules & Responsibilities

| Module | Source File | Core Functions & Responsibilities |
|--------|-------------|-----------------------------------|
| **Access Control** | `access-control/src/lib.rs` | RBAC contract defining roles (`Admin=0`, `Operator=1`, `Moderator=2`, `Oracle=3`). Implements safe two-step admin transfer (`propose_new_admin`, `accept_admin_role`) and final admin revocation guard. |
| **Admin & Governance** | `predifi-contract/src/admin.rs` | Protocol initialization with 6 parameters (`init`), emergency pausing (`pause`, `unpause`), 7-day timelocked fee adjustments (`set_fee_bps`, `apply_fee_bps`), token whitelisting, and contract upgrade logic. |
| **Pool Lifecycle** | `predifi-contract/src/pool.rs` | Market pool creation (`create_pool`), staking closure (`close_staking`), cancellation (`cancel_pool`, `emergency_cancel_pool`), and multi-operator consensus resolution (`resolve_pool`). |
| **Prediction & Staking** | `predifi-contract/src/prediction.rs` | Staking execution with token transfer (`place_prediction`), single claim (`claim_winnings`), full principal refunds (`claim_refund`), and multi-market batch claims (`batch_claim_winnings`). |
| **Payouts & Settlement** | `predifi-contract/src/payouts.rs` | Pure mathematical routines: `calculate_claim_payout`, `calculate_referral_amount`, `calculate_odds_bps`. Enforces protocol-favor rounding and Invariant INV-4. |
| **Treasury & Accrual** | `predifi-contract/src/treasury.rs` | Tracks accumulated protocol fees in contract instance storage; provides privileged withdrawal (`withdraw_treasury`) to authorized recipients. |
| **Referrals & Affiliates** | `predifi-contract/src/referral.rs` | Tracks referrer attribution (`Referrer(user, pool)`), accumulates affiliate volume, and routes referral fee shares upon winning claims. |
| **Price Feed & Oracle** | `predifi-contract/src/oracle.rs`, `price_feed.rs` | Pyth Network adapter integration, price feed freshness validation (`is_price_valid`), threshold evaluation (`evaluate_price_condition`), and automated pool resolution (`resolve_pool_from_price`). |
| **Safe Math** | `predifi-contract/src/safe_math.rs` | Arithmetic safety primitives (`checked_add`, `checked_mul`, `checked_div`, `percentage`, `proportion`). |

### Ledger Storage Architecture

Soroban provides three storage tiers with distinct durability and rent profiles:

1. **Instance Storage (Contract-Global)**:
   - Contains immutable configuration: `Config` (treasury address, fee basis points, resolution delay, min duration, max predictions).
   - Operational flags: `Paused` boolean, `TokenWhitelist` list, `PendingFeeBps` proposal with timestamp.
2. **Persistent Storage (Per-Entity Data)**:
   - Market pools: `Pool(pool_id) -> Pool`.
   - Stake records: `Pred(user, pool_id) -> UserPrediction`, `OutStakes(pool_id) -> Vec<i128>`.
   - Claims: `Claimed(user, pool_id) -> bool` (write-once duplicate claim guard).
   - Pagination indices: `UsrPrdCnt`, `UsrPrdIdx`, `ActivePool`, `CatPoolCt`.
   - *TTL Strategy*: Every read/write extends key durability by $535,680$ ledgers (~30 days). Permissionless `renew_storage_ttl(pool_id)` allows community maintenance.
3. **Temporary Storage (Transient State)**:
   - Reentrancy locks: `RentGuard` active flag.
   - Operator voting records: `ResVote(pool_id, operator)` auto-expire after resolution window.

---

## Backend API Flow

The backend service is an Axum web server orchestrating public reads, private administrative endpoints, real-time event streaming, and indexer ingestion.

### Request Lifecycle Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Client as Frontend / API Client
    participant CORS as CORS & RequestLogger Middleware
    participant RateLimiter as Governor Rate Limiter
    participant JWT as JWT Authenticator
    participant Handler as Route Handler
    participant Cache as Memory / Redis Cache
    participant DB as PostgreSQL Database
    participant RPC as Stellar Horizon / RPC

    Client->>CORS: HTTP Request (Method, Headers, Body)
    CORS->>CORS: Validate Origin, attach Request ID UUID
    CORS->>RateLimiter: Check IP Token Bucket
    alt Bucket Empty (Exceeded Quota)
        RateLimiter-->>Client: 429 Too Many Requests (Retry-After)
    else Tokens Available
        RateLimiter->>JWT: Verify Auth Header (if protected route)
        alt Token Expired / Invalid
            JWT-->>Client: 401 Unauthorized / 403 Forbidden
        else Authenticated / Public Route
            JWT->>Handler: Dispatch Request with AppState
            Handler->>Cache: Check In-Memory / Redis Cache
            alt Cache Hit
                Cache-->>Handler: Return Cached Payload
            else Cache Miss
                Handler->>DB: Query Indexed Tables (SQLx)
                opt Fallback or Live Verification Required
                    Handler->>RPC: Read Raw Soroban Ledger
                    RPC-->>Handler: Return Ledger Entry
                end
                DB-->>Handler: Return Row Set
                Handler->>Cache: Populate Cache with TTL
            end
            Handler-->>Client: 200 OK (Standard JSON ApiResponse Envelope)
        end
    end
```

### Backend Subsystems & State Structure

The application state (`AppState`) is cloned across worker threads via `Arc`:

```rust
pub struct AppState {
    pub config: Arc<Config>,
    pub cache: PriceCache,
    pub pool_cache: PoolCache,
    pub redis: RedisCache,
    pub db: Option<PgPool>,
    pub metrics: SharedMetrics,
    pub event_bus: EventBus,
}
```

```mermaid
flowchart LR
    subgraph AppStateComponents["Shared AppState (Arc)"]
        Config["Configuration (Arc<Config>)"]
        PoolCache["PoolCache (Negative TTL Support)"]
        PriceCache["PriceCache (Live Price Feeds)"]
        RedisCache["Redis Cache Client"]
        PgPool["PostgreSQL Connection Pool"]
        SharedMetrics["Prometheus Metrics Registry"]
        EventBus["Broadcast EventBus (Cap=256)"]
    end

    subgraph Workers["Background Workers"]
        StellarListener["Stellar Event Listener"]
        PriceFeedPoller["Oracle Price Poller"]
    end

    StellarListener --> PgPool
    StellarListener --> RedisCache
    StellarListener --> EventBus
    PriceFeedPoller --> PriceCache
```

### Caching Hierarchy & Invalidation Strategy

1. **L1 In-Memory Caches**:
   - `PoolCache`: Retains recent pool metadata and live odds. Implements negative caching for non-existent pool queries to prevent database cache stampedes.
   - `PriceCache`: Thread-safe read/write buffer for asset prices updated by oracle pollers.
2. **L2 Distributed Cache (Redis)**:
   - Caches serialized pool listings, category indexes, leaderboard rankings, and stats summaries.
   - Keys: `predifi:pools:active`, `predifi:stats:summary`, `predifi:leaderboard:volume`.
3. **Cache Invalidation**:
   - Ingesting a `prediction_placed` event immediately calls `state.redis.invalidate_stats_cache()`.
   - Resolving a pool purges `PoolCache` and triggers recalculation of leaderboard stats.

---

## Frontend State Management

The frontend architecture combines Next.js 15 App Router server-side capabilities with responsive client-side reactive state.

### State Management Architecture Diagram

```mermaid
flowchart TD
    subgraph BrowserEnvironment["Browser / Client Environment"]
        subgraph UIComponents["React Components (Next.js 15)"]
            MarketPages["Market Detail & Listing Pages"]
            BetModal["Prediction Staking Modal"]
            UserDashboard["User Portfolio & Stats"]
        end

        subgraph ReactiveState["State & Cache Management"]
            SWRLayer["SWR Data Cache (Deduped, Revalidate)"]
            WalletContext["Wallet Context (Freighter / Address / Balance)"]
            OptimisticUI["Optimistic State Machine"]
        end

        subgraph ClientPersistence["Storage Persistence Fallback"]
            HttpOnlyCookie["1. HttpOnly Secure Cookies"]
            DocumentCookie["2. document.cookie"]
            LocalStorageFallback["3. localStorage (Privacy Mode Fallback)"]
        end
    end

    subgraph ExternalServices["External Endpoints"]
        BackendAPI["PrediFi Backend API (REST/WSS)"]
        StellarRPCNode["Stellar Soroban RPC Node"]
    end

    MarketPages <--> SWRLayer
    BetModal <--> WalletContext
    BetModal --> OptimisticUI
    OptimisticUI --> SWRLayer

    WalletContext <--> ClientPersistence
    SWRLayer <-- "GET /api/v1/..." --> BackendAPI
    SWRLayer <-- "WebSocket Stream" --> BackendAPI
    WalletContext -- "Build & Submit TX" --> StellarRPCNode
```

### Client-Side Data Flow & Cache Strategy

- **SWR (Stale-While-Revalidate)**: Drives read queries (`usePools`, `usePoolDetails`, `useMarketPredictions`). Configured with automatic deduplication within 2-second windows and live background revalidation upon window focus.
- **Optimistic Updates**: When a prediction is submitted, the frontend updates user balances and implied outcome odds optimistically while awaiting on-chain confirmation. If the transaction fails during simulation or submission, state rolls back automatically.

### Session & Persistence Fallback Layer

To guarantee functionality across diverse mobile browsers and strict privacy modes (e.g. Brave shields, Safari ITP):
1. **Primary**: Tries reading server-issued `HttpOnly` secure session cookies.
2. **Secondary**: Reads client-accessible `document.cookie`.
3. **Fallback**: If cookies are blocked, falls back to `localStorage` with AES-GCM encrypted session blobs for persistent address and preferences storage.

### Transaction Signing & Simulation Lifecycle

Before requesting user signature in Freighter, all contract interactions execute the standard Soroban simulation protocol:

```mermaid
sequenceDiagram
    autonumber
    participant UI as Staking UI
    participant SDK as Stellar SDK
    participant RPC as Stellar RPC
    participant Wallet as Freighter Extension
    participant Network as Stellar Ledger

    UI->>SDK: Build place_prediction Transaction
    SDK->>RPC: simulateTransaction(tx)
    RPC-->>SDK: Simulation Response (Gas, Footprint, Auth, Result)
    alt Simulation Failed (Contract Error #Code)
        SDK-->>UI: Throw Decoded Error (e.g. "PredictionTooLate #43")
        UI-->>UI: Display Actionable Error Toast to User
    else Simulation Succeeded
        SDK->>Wallet: requestSignTransaction(simulatedTx)
        Wallet-->>UI: Prompt User Approval Modal
        alt User Rejects
            Wallet-->>UI: Transaction Rejected by User
        else User Signs
            Wallet-->>SDK: Return Signed Transaction Envelope XDR
            SDK->>RPC: sendTransaction(signedTx)
            RPC-->>SDK: Transaction Hash (Pending Status)
            SDK->>RPC: getTransaction(hash) (Poll until Success)
            RPC-->>SDK: Final Ledger Status: SUCCESS
            SDK-->>UI: Confirmation Alert & Live Odds Refresh
        end
    end
```

---

## End-to-End Data Flow

### 1. Pool Creation Flow

```mermaid
sequenceDiagram
    autonumber
    participant Creator as Pool Creator
    participant Contract as PrediFi Contract
    participant Horizon as Stellar Horizon / Events
    participant Indexer as Backend Indexer Worker
    participant DB as Postgres
    participant Client as Web Clients

    Creator->>Contract: create_pool(title, end_time, options, token, stake_limits)
    Contract->>Contract: Verify options count, duration, whitelisted token
    Contract->>Contract: Write Pool to Persistent Storage & Emit PoolCreatedEvent
    Horizon->>Indexer: Detect PoolCreatedEvent on Ledger
    Indexer->>DB: INSERT into pools table
    Indexer->>Client: Broadcast pool_created on WebSocket
```

### 2. Prediction Staking & Event Indexing Flow

```mermaid
sequenceDiagram
    autonumber
    participant User as Bettor
    participant SAC as Token SAC Contract
    participant Predifi as PrediFi Contract
    participant Worker as Stellar Listener Worker
    participant EventBus as WebSocket EventBus
    participant Dashboard as Live Dashboard Client

    User->>Predifi: place_prediction(pool_id, outcome, amount, referrer)
    Predifi->>Predifi: Check RentGuard, cooldown, and limits
    Predifi->>SAC: transfer(from: User, to: Predifi, amount)
    SAC-->>Predifi: Token Transfer Success
    Predifi->>Predifi: Update OutStakes & emit PredictionPlacedEvent
    Worker->>Worker: Ingest event from RPC stream
    Worker->>EventBus: event_bus.send(prediction_placed)
    EventBus-->>Dashboard: Push JSON event over WebSocket
    Dashboard->>Dashboard: Increment live volume & re-render odds chart
```

### 3. Oracle Price Updates & Automated Resolution

```mermaid
sequenceDiagram
    autonumber
    participant Pyth as Pyth Network
    participant Feeder as Oracle Bot
    participant Contract as PrediFi Contract
    participant Resolver as Any Caller / Operator

    Feeder->>Contract: update_price_feed(feed_pair, price, conf, timestamp)
    Contract->>Contract: Store fresh price in Persistent Storage
    Note over Contract: Pool end_time + resolution_delay elapses
    Resolver->>Contract: resolve_pool_from_price(pool_id)
    Contract->>Contract: Evaluate price condition against cached feed
    Contract->>Contract: Set pool state to Resolved (winning outcome selected)
    Contract->>Contract: Emit PoolResolvedEvent
```

### 4. Payout Settlement & Fee Routing Flow

```mermaid
sequenceDiagram
    autonumber
    participant Winner as Winning Bettor
    participant Contract as PrediFi Contract
    participant Payouts as payouts.rs Math
    participant Treasury as Protocol Treasury
    participant Referrer as Affiliate Wallet
    participant SAC as Token SAC Contract

    Winner->>Contract: claim_winnings(pool_id)
    Contract->>Contract: Check Claimed(user, pool_id) == false
    Contract->>Payouts: calculate_claim_payout(user_stake, win_pool, total_pool, fee_bps)
    Payouts-->>Contract: Net Payout, Protocol Fee, Referral Share
    Contract->>Contract: Mark Claimed = true
    Contract->>SAC: transfer(to: Winner, amount: Net Payout)
    Contract->>SAC: transfer(to: Referrer, amount: Referral Share)
    Contract->>Treasury: Accrue Protocol Fee
    Contract->>Contract: Emit WinningsClaimedEvent & ReferralPaidEvent
```

---

## Prediction Market Lifecycle State Machine

A prediction pool progresses through strictly regulated, monotonic states:

```mermaid
stateDiagram-v2
    [*] --> Active: create_pool()
    note right of Active
        - Staking open
        - Token whitelist enforced
        - Cooldown between bets enforced
    end note

    Active --> Active: place_prediction()
    Active --> StakingClosed: Ledger timestamp >= pool.end_time
    note right of StakingClosed
        - Predictions blocked
        - Resolution delay countdown active
    end note

    Active --> Canceled: cancel_pool() / emergency_cancel_pool()
    StakingClosed --> Canceled: emergency_cancel_pool() (multisig)

    StakingClosed --> Resolving: resolution_delay elapsed
    Resolving --> Resolved: resolve_pool() [quorum] / resolve_pool_from_price() [oracle]
    Resolving --> Disputed: Moderator flags dispute

    Disputed --> Resolved: Arbitrator resolution decision
    Disputed --> Canceled: Arbitrator cancellation decision

    Resolved --> Claimed: claim_winnings() [Winners withdraw]
    Canceled --> Refunded: claim_refund() [Full principal returned]

    Claimed --> [*]
    Refunded --> [*]
```

---

## Security Invariants & Reentrancy Guards

PrediFi enforces eight formal protocol invariants verified in unit and integration test suites:

| Invariant | Specification | Enforcement Mechanism |
|-----------|---------------|-----------------------|
| **INV-1** | $\text{Pool Total Stake} = \sum_{i=0}^{N-1} \text{Outcome Stake}_i$ | Checked arithmetic on every `place_prediction`. |
| **INV-2** | No payouts exceed total pool stake vault balance. | Bounded payouts in `payouts.rs` with `RoundingMode::ProtocolFavor`. |
| **INV-3** | Double claims are impossible. | Write-once `Claimed(user, pool_id)` persistent storage key. |
| **INV-4** | $\text{Protocol Fee} + \text{Referral Cut} + \text{Net Winnings} \le \text{Gross Payout}$. | Explicit conservation check prior to token disbursements. |
| **INV-5** | State transitions are irreversible (monotonic). | `MarketState` machine guards (`Active -> Resolved/Canceled`). |
| **INV-6** | Protocol fee basis points $\le 10,000$ ($100\%$). | Parameter validation on `init` and `set_fee_bps`. |
| **INV-7** | Reentrancy attacks are impossible. | `RentGuard` lock acquisition on `place_prediction` and `claim_winnings`. |
| **INV-8** | End time must be strictly greater than creation timestamp. | Minimum pool duration constraint (`min_pool_duration`). |

---

## Monitoring, Telemetry & Observability

```mermaid
flowchart LR
    subgraph Signals["Observability Signals"]
        MetricsEndpoint["GET /metrics (Prometheus)"]
        TraceLogger["OpenTelemetry Tracing / JSON Logger"]
        HorizonEvents["Stellar Horizon Event Stream"]
    end

    subgraph Aggregators["Monitoring Infrastructure"]
        PrometheusServer["Prometheus Server"]
        LokiGrafana["Grafana & Loki Dashboard"]
        PagerDutyAlerts["PagerDuty On-Call Alerts"]
    end

    MetricsEndpoint --> PrometheusServer
    TraceLogger --> LokiGrafana
    HorizonEvents --> PagerDutyAlerts
    PrometheusServer --> LokiGrafana
```

### Metrics & Diagnostic Endpoints

- **`/metrics`**: Exports real-time Prometheus counters and gauges:
  - `http_requests_total{method, path, status}`
  - `http_request_duration_seconds{path}`
  - `db_pool_active_connections`, `db_pool_idle_connections`
  - `websocket_active_clients`
  - `cache_hits_total{cache="pool"|"price"|"redis"}`
- **`/health`**: Evaluates individual subsystem health (`db`, `rpc`, `redis`, `price_cache`). Returns 200 OK if all operational, or 503 Service Unavailable with per-dependency diagnostics.
- **Contract Event Monitoring**: Scrapers watch for high-priority security topics: `unauthorized_resolution`, `unauthorized_admin_op`, `double_claim_attempt`, `contract_paused_alert`.

---

**Document Maintained By**: Web3Novalabs Core Engineering  
**Source Code References**: [predifi-contract](file:///Users/victor/Desktop/Pvsaint/predifi/contract/contracts/predifi-contract/src/lib.rs), [access-control](file:///Users/victor/Desktop/Pvsaint/predifi/contract/contracts/access-control/src/lib.rs), [backend server.rs](file:///Users/victor/Desktop/Pvsaint/predifi/backend/src/server.rs), [frontend api client](file:///Users/victor/Desktop/Pvsaint/predifi/frontend/lib/api/pools.ts)
