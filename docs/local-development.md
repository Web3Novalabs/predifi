# Local Development Environment

The repository-root `docker-compose.yml` brings up every service PrediFi needs
for local development: PostgreSQL, Redis, a local Stellar network, the Axum
backend and the Next.js dev server. An optional profile adds a Loki-based log
aggregation stack.

## Prerequisites

- Docker Engine 24+ with the Compose v2 plugin (`docker compose version`)
- Roughly 8 GB of free disk space — the Stellar quickstart image and the Rust
  build cache are the bulk of it

## Starting the stack

```bash
docker compose up
```

| Service    | URL                     | Notes                                        |
| ---------- | ----------------------- | -------------------------------------------- |
| `frontend` | <http://localhost:3001> | Next.js dev server with hot reload            |
| `backend`  | <http://localhost:3000> | Axum API; `/health` for readiness             |
| `postgres` | `localhost:5432`        | user/password/database: `postgres`/`postgres`/`predifi` |
| `redis`    | `localhost:6379`        |                                              |
| `stellar`  | <http://localhost:8000> | Local network; Soroban RPC at `/rpc`          |

The first `docker compose up` is slow: the Rust dependency tree is compiled
from scratch and the Stellar quickstart image initialises its ledger. Both are
cached in named volumes, so subsequent starts are fast.

### Live reload

Both application services bind-mount their source directory:

- `backend` runs under `cargo watch` and rebuilds when `backend/src` or
  `backend/Cargo.toml` changes. On macOS and Windows, bind mounts do not
  propagate inotify events reliably — append `--poll` to the `cargo watch`
  command in `backend/Dockerfile.dev` if rebuilds do not trigger.
- `frontend` runs `next dev` with `WATCHPACK_POLLING=true` already set.

`node_modules`, `.next` and `target` live in named volumes so host artifacts
never shadow the container's.

### Seed data

The `db-seed` service runs the sqlx migrations and inserts deterministic sample
data before the backend starts. It is idempotent (every insert uses
`ON CONFLICT`), so it re-runs safely on every `up`.

To wipe and re-seed:

```bash
docker compose run --rm db-seed cargo run --bin predifi-seed -- --fresh
```

### Resetting

```bash
docker compose down -v   # removes containers and all named volumes
```

## Log aggregation

The `logging` profile adds Loki (storage and query), Promtail (shipping) and
Grafana (dashboards and alerts):

```bash
docker compose --profile logging up
```

Grafana is at <http://localhost:3002> (`admin` / `admin`) with the Loki data
source pre-provisioned and the **PrediFi — Logs & Error Trends** dashboard in
the *PrediFi* folder.

### How logs flow

1. The backend logs structured JSON through `tracing-subscriber`.
2. Promtail discovers every container in this compose project through the
   Docker daemon and ships their stdout/stderr to Loki.
3. Backend lines are parsed as JSON, so `level` and `span_name` become Loki
   labels and the rendered line is just the message. Other services are shipped
   verbatim.

Useful LogQL queries:

```logql
{job="predifi", service="backend", level="ERROR"}
sum by (service) (rate({job="predifi", level="ERROR"}[5m]))
{job="predifi"} |= "pool_id"
```

### Alerting

`docker/loki/rules/fake/predifi-log-alerts.yml` defines log-based alerts —
error-rate spikes, Rust panics, repeated database errors, and a dead-man's
switch for missing backend logs. The Loki ruler evaluates them and Grafana
surfaces them under **Alerting → Alert rules** (the data source is provisioned
with `manageAlerts: true`).

The `fake` directory name is Loki's tenant ID when `auth_enabled` is false; it
is not a placeholder.

No Alertmanager is included, so alerts are visible but not routed. Point the
ruler at one by adding `alertmanager_url` to `docker/loki/loki-config.yml`.

## Configuration files

| Path                                                  | Purpose                          |
| ----------------------------------------------------- | -------------------------------- |
| `docker-compose.yml`                                   | Service definitions              |
| `backend/Dockerfile.dev`, `frontend/Dockerfile.dev`    | Development images               |
| `docker/loki/loki-config.yml`                          | Loki single-binary configuration |
| `docker/loki/rules/fake/predifi-log-alerts.yml`        | Log-based alert rules            |
| `docker/promtail/promtail-config.yml`                  | Log shipping and JSON parsing    |
| `docker/grafana/provisioning/`                         | Data source and dashboard wiring |
| `docker/grafana/dashboards/predifi-logs.json`          | Error trend dashboard            |
