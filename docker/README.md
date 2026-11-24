# Observability stack (Grafana, Loki, Promtail)

Local log aggregation for PrediFi. Config lives in this directory; the
services themselves are defined in the repository-root `docker-compose.yml`
under the `logging` profile.

This stack is for local development only. Do not reuse these configs in
production.

## What runs

| Service   | Image                     | Host port | Role |
|-----------|---------------------------|-----------|------|
| Loki      | `grafana/loki:3.3.2`      | `3100`    | Stores and queries logs |
| Promtail  | `grafana/promtail:3.3.2`  | —         | Ships container stdout/stderr into Loki |
| Grafana   | `grafana/grafana:11.4.0`  | `3002`    | Dashboards and alert UI |

Promtail discovers every container in the `predifi` Compose project via the
Docker socket and labels lines with `job=predifi` plus the Compose service
name (`backend`, `frontend`, …). Backend JSON logs are parsed so `level` and
`span_name` become Loki labels; other services are shipped as plain text.

## Config files

| Path | What it does |
|------|----------------|
| `loki/loki-config.yml` | Single-binary Loki: filesystem storage, 7-day retention, ruler API enabled. `auth_enabled` is false. |
| `loki/rules/fake/predifi-log-alerts.yml` | Log-based alerts (error-rate spike, panic, DB errors, missing backend logs). `fake` is Loki's tenant id when auth is off — not a placeholder. |
| `promtail/promtail-config.yml` | Docker service discovery + JSON parsing for `backend` / `backend-seed`. Pushes to `http://loki:3100/loki/api/v1/push`. |
| `grafana/provisioning/datasources/loki.yml` | Provisions the Loki datasource (`uid: predifi-loki`) at `http://loki:3100`, with `manageAlerts: true`. |
| `grafana/provisioning/dashboards/predifi-logs.yml` | File provider that loads dashboards from `/var/lib/grafana/dashboards` into the **PrediFi** folder. |
| `grafana/dashboards/predifi-logs.json` | The **PrediFi — Logs & Error Trends** dashboard (`uid: predifi-logs`). |

Compose mounts these files read-only into the containers (see the `loki`,
`promtail`, and `grafana` services in `docker-compose.yml`).

## Start the stack

From the **repository root** (the directory that contains `docker-compose.yml`):

```bash
# App services + Loki, Promtail, and Grafana
docker compose --profile logging up
```

To attach logging to an already-running app stack:

```bash
docker compose --profile logging up loki promtail grafana
```

Wait until Grafana is up (`docker compose --profile logging ps` shows
`predifi-grafana` healthy / running). First start only pulls the three images.

Stop with `Ctrl+C`, or `docker compose --profile logging down`. Named volumes
(`loki-data`, `grafana-data`, `promtail-positions`) keep history across
restarts; add `-v` to wipe them.

## Open Grafana and the dashboard

1. Open [http://localhost:3002](http://localhost:3002)
2. Sign in with **admin** / **admin** (set by `GF_SECURITY_ADMIN_USER` /
   `GF_SECURITY_ADMIN_PASSWORD` in `docker-compose.yml`). Grafana will prompt
   you to change the password; you can skip that locally.
3. Open **Dashboards → PrediFi → PrediFi — Logs & Error Trends**, or go
   directly to [http://localhost:3002/d/predifi-logs](http://localhost:3002/d/predifi-logs)

The Loki datasource is already provisioned. Alerts from
`loki/rules/fake/predifi-log-alerts.yml` show up under **Alerting → Alert
rules**. There is no Alertmanager in this compose file, so alerts are visible
but not routed.

## Where `predifi-logs` comes from

The dashboard is **not** created by hand in the Grafana UI. It is the JSON
file `grafana/dashboards/predifi-logs.json` (`"uid": "predifi-logs"`).

On container start, Grafana reads `grafana/provisioning/dashboards/predifi-logs.yml`,
which points at `/var/lib/grafana/dashboards` — that path is a bind-mount of
`docker/grafana/dashboards/`. Grafana imports the JSON into the **PrediFi**
folder and refreshes it every 30 seconds.

Edits to the JSON on disk show up after the next provider refresh. UI edits
are allowed (`allowUiUpdates: true`) but will be overwritten if the file
changes.
