# PrediFi Monitoring & Alert Rules

This document details the alert rules configured for the PrediFi monitoring stack across Prometheus and Grafana.

All alerts are provisioned strictly as code:
- **Prometheus Alert Rules**: [`backend/grafana/alerts/predifi_alerts.yml`](file:///Users/mac/Documents/OPENSOURCE/Drips/zaps/predifi/backend/grafana/alerts/predifi_alerts.yml) (and Terraform deployment via [`terraform/modules/monitoring/main.tf`](file:///Users/mac/Documents/OPENSOURCE/Drips/zaps/predifi/terraform/modules/monitoring/main.tf)).
- **Grafana Provisioned Rules**: [`docker/grafana/provisioning/alerting/rules.yaml`](file:///Users/mac/Documents/OPENSOURCE/Drips/zaps/predifi/docker/grafana/provisioning/alerting/rules.yaml).

---

## Core Golden Signal Alerts

### 1. API Error Rate (`HighApiErrorRate`)

- **PromQL**:
  ```promql
  (sum(rate(app_http_requests_total{status=~"5.."}[5m])) / sum(rate(app_http_requests_total[5m]))) * 100 > 1
  ```
- **Severity**: `critical`
- **Threshold**: `> 1%` over 5 minutes (evaluated for 2 minutes)
- **Rationale**:
  In a production financial and prediction market platform, HTTP 5xx responses represent unhandled application errors, database disconnections, or backend panics. A sustained error rate greater than 1% indicates a critical failure mode that directly disrupts users placing predictions or claiming payouts. The 2-minute duration avoids alerting on momentary transient spikes during deploys while quickly triggering on real regressions.
- **Remediation**:
  1. Inspect backend error logs via Loki: `{job="predifi", service="backend", level="ERROR"}`.
  2. Verify database and Redis connectivity.
  3. Check recent deployments or contract address migrations.

---

### 2. API Request Latency (`HighP99Latency`)

- **PromQL**:
  ```promql
  histogram_quantile(0.99, sum(rate(app_http_request_duration_seconds_bucket[5m])) by (le)) > 0.5
  ```
- **Severity**: `warning`
- **Threshold**: `> 500ms` (0.5s) at the 99th percentile over 5 minutes (evaluated for 3 minutes)
- **Rationale**:
  Prediction markets require near real-time execution. When 99% of requests exceed 500ms, user interface responsiveness degrades noticeably, and automated traders face slippage and stale odds. A 3-minute sustained window ensures that isolated long-running export queries do not trigger false positives, while persistent slowdowns from unindexed database queries or slow Soroban RPC calls are surfaced.
- **Remediation**:
  1. Identify high-latency endpoints: `topk(5, sum by (path) (rate(app_http_request_duration_seconds_sum[5m]) / rate(app_http_request_duration_seconds_count[5m])))`.
  2. Inspect database query latency metrics: `app_db_query_duration_seconds`.
  3. Check Stellar RPC node health and response times.

---

### 3. Worker Queue Depth & DLQ Accumulation

#### A. Worker Queue Backlog (`WorkerQueueBacklog`)
- **PromQL**:
  ```promql
  app_worker_queue_depth > 50
  ```
- **Severity**: `warning`
- **Threshold**: `> 50` pending/retrying jobs (evaluated for 3 minutes)
- **Rationale**:
  The background worker pipeline handles asynchronous event ingestion from the Stellar network, payout disbursements, and pool resolution updates. Under steady state, jobs are processed within milliseconds of enqueueing. A backlog greater than 50 jobs lasting more than 3 minutes indicates worker starvation, upstream RPC throttling, or excessive retry loops.
- **Remediation**:
  1. Check worker logs for retry backoff reasons.
  2. Monitor Soroban RPC rate limits and response codes.
  3. Verify worker concurrency settings.

#### B. Worker DLQ Backlog (`WorkerDLQBacklog`)
- **PromQL**:
  ```promql
  app_worker_dlq_depth > 10
  ```
- **Severity**: `critical`
- **Threshold**: `> 10` dead-lettered jobs (evaluated for 2 minutes)
- **Rationale**:
  Jobs reach the dead-letter queue (DLQ) only after exhausting their retry budget (e.g. 5 consecutive attempts). Accumulation of more than 10 dead-lettered jobs indicates persistent failure (e.g. malformed contract events, invalid transaction signatures, or invariant violations) that requires manual operator review and replay via `/worker/requeue`.
- **Remediation**:
  1. Inspect dead-lettered entries via worker health API `/health` or management endpoints.
  2. Identify the root cause (e.g. contract revert error code).
  3. Apply bug fix and replay failed jobs using the requeue tooling.

---

### 4. Database Connection Saturation

#### A. Warning Saturation (`DatabaseConnectionPoolSaturated`)
- **PromQL**:
  ```promql
  db_pool_utilization_ratio > 0.8
  ```
- **Severity**: `warning`
- **Threshold**: `> 80%` (0.8) connection pool utilization (evaluated for 2 minutes)
- **Rationale**:
  The PostgreSQL connection pool (`sqlx::PgPool`) has a fixed capacity to protect the database instance from connection exhaustion. When active connections exceed 80% of capacity for 2 minutes, the pool is running low on headroom. Concurrency spikes will cause incoming queries to block, increasing latency across all HTTP routes. Triggering at 80% gives operators advance warning to inspect connection leaks or scale pool capacity.

#### B. Critical Saturation (`DatabaseConnectionPoolCritical`)
- **PromQL**:
  ```promql
  db_pool_utilization_ratio > 0.95
  ```
- **Severity**: `critical`
- **Threshold**: `> 95%` (0.95) connection pool utilization (evaluated for 1 minute)
- **Rationale**:
  At >95% saturation, the pool is virtually exhausted. New HTTP requests requiring database access will immediately encounter connection checkout timeouts (default 30s) and return 500 errors. An immediate 1-minute alert triggers fast auto-scaling or traffic mitigation.
- **Remediation**:
  1. Check active database connections: `db_pool_active` vs `db_pool_size`.
  2. Identify long-running transactions: `SELECT pid, now() - xact_start AS duration, query FROM pg_stat_activity WHERE state != 'idle' ORDER BY duration DESC;`.
  3. Adjust `max_connections` or ASG scaling if sustained traffic increase is legitimate.

---

## Auxiliary Reliability Alerts

| Alert | Condition | Severity | Rationale |
|---|---|---|---|
| `DatabaseQueryFailures` | `sum(rate(app_db_queries_total{result="error"}[5m])) > 5` | `critical` | Spikes in database query errors point to schema mismatches, disk exhaustion, or deadlocks. |
| `HighRedisErrors` | `sum(rate(app_redis_operations_total{result="error"}[5m])) > 10` | `warning` | Redis failures cause session lookup fallbacks, cache misses, and degraded throughput. |
| `ContractInteractionFailures` | `sum(rate(app_db_queries_total{query_type=~".*contract.*",result="error"}[5m])) > 0` | `critical` | Soroban smart contract interaction errors risk halting betting and settlement workflows. |
| `BackendMemoryHigh` | `(app_memory_used_bytes / app_memory_total_bytes) * 100 > 90` | `critical` | Memory utilization above 90% risks process termination by Linux Out-Of-Memory (OOM) killer. |
