//! Component-level health checks (#1946).
//!
//! `GET /health` and `GET /api/v1/health` both run [`check_all`], which probes
//! every dependency concurrently and reports each as `healthy`, `degraded`
//! or `unhealthy`:
//!
//! | Component | `healthy` | `degraded` | `unhealthy` |
//! |---|---|---|---|
//! | `database` | `SELECT 1` succeeds | slow ping, or connection pool saturated | ping fails or times out |
//! | `redis` | `PING` succeeds | slow ping, or down/unconfigured while `PREDIFI_REDIS_REQUIRED=false` (reads fall back to the DB) | down/unconfigured while `PREDIFI_REDIS_REQUIRED=true` |
//! | `stellar_rpc` | `getHealth` reports `healthy` | slow, or node reports another status | unreachable after retries |
//! | `disk` | ≥ 10 % free | < 10 % free, or usage unknown | < 5 % free |
//! | `price_cache` | populated | — | empty |
//!
//! The overall status is the worst component status. Only `unhealthy` maps
//! to HTTP 503, so load balancers keep routing to a degraded instance.
//!
//! The response keeps the pre-#1946 `status` / `dependencies` / `errors`
//! fields for existing monitors and adds `health` and `components`:
//!
//! ```json
//! {
//!   "status": "degraded",
//!   "health": "degraded",
//!   "service": "predifi-backend",
//!   "version": "0.1.0",
//!   "checked_at": "2026-10-17T12:00:00Z",
//!   "duration_ms": 41,
//!   "components": {
//!     "database":    { "status": "healthy",  "critical": true,  "latency_ms": 2,
//!                      "details": { "pool_size": 10, "pool_idle": 9, "pool_utilization": 0.1 } },
//!     "redis":       { "status": "degraded", "critical": false, "latency_ms": 0,
//!                      "message": "Redis ping failed; serving uncached reads from the database" },
//!     "stellar_rpc": { "status": "healthy",  "critical": true,  "latency_ms": 38,
//!                      "details": { "node_status": "healthy", "latest_ledger": 123456, "attempts": 1 } },
//!     "disk":        { "status": "healthy",  "critical": false,
//!                      "details": { "mount_point": "/", "available_bytes": 1, "total_bytes": 2, "free_ratio": 0.5 } },
//!     "price_cache": { "status": "healthy",  "critical": true, "details": { "assets": 3 } }
//!   },
//!   "dependencies": { "db": "ok", "rpc": "ok", "redis": "unreachable", "price_cache": "ok", "disk": "ok" },
//!   "errors": { "db": null, "rpc": null, "redis": "Redis ping failed; ...", "price_cache": null, "disk": null }
//! }
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;

use crate::config::Config;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;

/// Database ping timeout.
const DB_TIMEOUT: Duration = Duration::from_secs(3);
/// Database pings slower than this are reported as degraded.
const DB_SLOW: Duration = Duration::from_millis(500);
/// Pool utilisation at or above this (with no idle connection) is degraded.
const DB_POOL_SATURATION: f64 = 0.9;
/// Redis ping timeout.
const REDIS_TIMEOUT: Duration = Duration::from_secs(2);
/// Redis pings slower than this are reported as degraded.
const REDIS_SLOW: Duration = Duration::from_millis(100);
/// Stellar RPC calls slower than this are reported as degraded.
const RPC_SLOW: Duration = Duration::from_secs(1);
/// Free-space ratio below which the disk is degraded.
const DISK_DEGRADED_FREE_RATIO: f64 = 0.10;
/// Free-space ratio below which the disk is unhealthy.
const DISK_UNHEALTHY_FREE_RATIO: f64 = 0.05;

/// Health of one component, ordered from best to worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    /// The pre-#1946 top-level `status` value.
    fn legacy(self) -> &'static str {
        match self {
            Self::Healthy => "ok",
            Self::Degraded => "degraded",
            Self::Unhealthy => "error",
        }
    }
}

/// Result of probing a single component.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    /// Whether this component failing makes the whole service unusable.
    pub critical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Pre-#1946 dependency string (`ok`, `unreachable`, `not_configured`, …).
    #[serde(skip)]
    legacy: &'static str,
}

impl ComponentHealth {
    fn new(status: HealthStatus, critical: bool, legacy: &'static str) -> Self {
        Self {
            status,
            critical,
            latency_ms: None,
            message: None,
            details: None,
            legacy,
        }
    }

    fn latency(mut self, elapsed: Duration) -> Self {
        self.latency_ms = Some(elapsed.as_millis() as u64);
        self
    }

    fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    fn details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Value for the legacy `errors` block: the message for real failures,
    /// `null` for healthy or merely unconfigured components.
    fn legacy_error(&self) -> Option<&str> {
        match self.legacy {
            "ok" | "not_configured" => None,
            _ => self.message.as_deref(),
        }
    }
}

/// Every component's health plus the aggregated status.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub components: BTreeMap<&'static str, ComponentHealth>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
}

impl HealthReport {
    fn from_components(
        components: BTreeMap<&'static str, ComponentHealth>,
        start: Instant,
    ) -> Self {
        let status = components
            .values()
            .map(|c| c.status)
            .max()
            .unwrap_or(HealthStatus::Healthy);
        Self {
            status,
            components,
            checked_at: chrono::Utc::now(),
            duration: start.elapsed(),
        }
    }

    /// HTTP status for the report: 503 only when something is unhealthy.
    pub fn http_status(&self) -> StatusCode {
        if self.status == HealthStatus::Unhealthy {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        }
    }

    /// Render the JSON body. `version` is the crate version for `/health`
    /// and `"v1"` for `/api/v1/health`.
    pub fn to_json(&self, version: &str) -> serde_json::Value {
        let legacy = |name: &str| self.components.get(name).map(|c| c.legacy);
        let error = |name: &str| self.components.get(name).and_then(|c| c.legacy_error());
        json!({
            "status": self.status.legacy(),
            "health": self.status,
            "service": "predifi-backend",
            "version": version,
            "checked_at": self.checked_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "duration_ms": self.duration.as_millis() as u64,
            "components": self.components,
            "dependencies": {
                "db": legacy("database"),
                "rpc": legacy("stellar_rpc"),
                "redis": legacy("redis"),
                "price_cache": legacy("price_cache"),
                "disk": legacy("disk"),
            },
            "errors": {
                "db": error("database"),
                "rpc": error("stellar_rpc"),
                "redis": error("redis"),
                "price_cache": error("price_cache"),
                "disk": error("disk"),
            },
        })
    }

    /// Render the report as an HTTP response.
    pub fn into_response_with_version(self, version: &str) -> Response {
        (self.http_status(), Json(self.to_json(version))).into_response()
    }
}

/// Probe every component concurrently.
pub async fn check_all(
    config: &Config,
    db: Option<&sqlx::PgPool>,
    redis: &RedisCache,
    cache: &PriceCache,
) -> HealthReport {
    let start = Instant::now();
    let (database, redis, stellar_rpc) = tokio::join!(
        check_database(db),
        check_redis(redis, config.redis_required),
        check_stellar_rpc(
            &config.stellar_rpc_url,
            Duration::from_secs(config.rpc_health_timeout_secs),
            config.rpc_health_retry_count,
        ),
    );
    let disk = check_disk(&disk_probe_path());
    let price_cache = check_price_cache(cache);

    let components = BTreeMap::from([
        ("database", database),
        ("redis", redis),
        ("stellar_rpc", stellar_rpc),
        ("disk", disk),
        ("price_cache", price_cache),
    ]);
    let report = HealthReport::from_components(components, start);
    if report.status != HealthStatus::Healthy {
        let failing: Vec<&str> = report
            .components
            .iter()
            .filter(|(_, c)| c.status != HealthStatus::Healthy)
            .map(|(name, _)| *name)
            .collect();
        tracing::warn!(status = ?report.status, components = ?failing, "health check not healthy");
    }
    report
}

/// `SELECT 1` plus connection-pool saturation.
pub async fn check_database(db: Option<&sqlx::PgPool>) -> ComponentHealth {
    let Some(pool) = db else {
        return ComponentHealth::new(HealthStatus::Healthy, true, "not_configured")
            .message("database pool is not configured");
    };

    let start = Instant::now();
    let ping = tokio::time::timeout(
        DB_TIMEOUT,
        crate::metrics::observe_db("health_check", sqlx::query("SELECT 1").execute(pool)),
    )
    .await;
    let elapsed = start.elapsed();

    let snapshot = crate::db::metrics::snapshot(pool);
    let details = json!({
        "pool_size": snapshot.size,
        "pool_idle": snapshot.idle,
        "pool_active": snapshot.active,
        "pool_utilization": snapshot.utilization(),
    });

    let base = match ping {
        Ok(Ok(_)) => ComponentHealth::new(HealthStatus::Healthy, true, "ok"),
        Ok(Err(error)) => ComponentHealth::new(HealthStatus::Unhealthy, true, "unreachable")
            .message(format!("database ping failed: {error}")),
        Err(_) => ComponentHealth::new(HealthStatus::Unhealthy, true, "unreachable").message(
            format!("database ping timed out after {}s", DB_TIMEOUT.as_secs()),
        ),
    };
    let base = base.latency(elapsed).details(details);
    if base.status != HealthStatus::Healthy {
        return base;
    }

    if snapshot.idle == 0 && snapshot.utilization() >= DB_POOL_SATURATION {
        return ComponentHealth {
            status: HealthStatus::Degraded,
            ..base
        }
        .message("database connection pool is saturated");
    }
    if elapsed >= DB_SLOW {
        return ComponentHealth {
            status: HealthStatus::Degraded,
            ..base
        }
        .message(format!("database ping took {}ms", elapsed.as_millis()));
    }
    base
}

/// Redis `PING`. Redis is only critical when `redis_required` is set;
/// otherwise the service keeps working (uncached) and reports `degraded`.
pub async fn check_redis(redis: &RedisCache, redis_required: bool) -> ComponentHealth {
    let down = if redis_required {
        HealthStatus::Unhealthy
    } else {
        HealthStatus::Degraded
    };
    let fallback = if redis_required {
        ""
    } else {
        "; serving uncached reads from the database"
    };

    if !redis.is_available() {
        return ComponentHealth::new(down, redis_required, "not_configured")
            .message(format!("Redis is not configured{fallback}"));
    }

    let start = Instant::now();
    let ok = tokio::time::timeout(REDIS_TIMEOUT, redis.ping())
        .await
        .unwrap_or(false);
    let elapsed = start.elapsed();

    if !ok {
        return ComponentHealth::new(down, redis_required, "unreachable")
            .latency(elapsed)
            .message(format!("Redis ping failed{fallback}"));
    }
    let health = ComponentHealth::new(HealthStatus::Healthy, redis_required, "ok").latency(elapsed);
    if elapsed >= REDIS_SLOW {
        return ComponentHealth {
            status: HealthStatus::Degraded,
            ..health
        }
        .message(format!("Redis ping took {}ms", elapsed.as_millis()));
    }
    health
}

/// Stellar RPC `getHealth`, retried with exponential backoff (1s, 2s, … ≤ 5s).
pub async fn check_stellar_rpc(rpc_url: &str, timeout: Duration, retries: u8) -> ComponentHealth {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let max_attempts = u32::from(retries.max(1));
    let mut last_error = String::new();

    for attempt in 1..=max_attempts {
        let start = Instant::now();
        let request = crate::tracing_context::propagate_correlation_id(
            client
                .post(rpc_url)
                .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "getHealth" })),
        );
        match request.send().await {
            Ok(response) if response.status().is_success() => {
                let elapsed = start.elapsed();
                let body: serde_json::Value = response.json().await.unwrap_or_default();
                return rpc_component(&body, elapsed, attempt);
            }
            Ok(response) => last_error = format!("HTTP {} response", response.status()),
            Err(error) => last_error = error.to_string(),
        }
        if attempt < max_attempts {
            let backoff = 2u64.pow(attempt - 1).min(5);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
        }
    }

    ComponentHealth::new(HealthStatus::Unhealthy, true, "unreachable")
        .message(format!("Stellar RPC unreachable: {last_error}"))
        .details(json!({ "attempts": max_attempts }))
}

/// Interpret a successful `getHealth` response.
fn rpc_component(body: &serde_json::Value, elapsed: Duration, attempts: u32) -> ComponentHealth {
    let result = body.get("result");
    let node_status = result
        .and_then(|r| r.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut details = json!({ "node_status": node_status, "attempts": attempts });
    for (from, to) in [
        ("latestLedger", "latest_ledger"),
        ("oldestLedger", "oldest_ledger"),
        ("ledgerRetentionWindow", "ledger_retention_window"),
    ] {
        if let Some(value) = result.and_then(|r| r.get(from)) {
            details[to] = value.clone();
        }
    }

    let health = ComponentHealth::new(HealthStatus::Healthy, true, "ok")
        .latency(elapsed)
        .details(details);
    if node_status != "healthy" {
        return ComponentHealth {
            status: HealthStatus::Degraded,
            ..health
        }
        .message(format!("Stellar RPC node reports status '{node_status}'"));
    }
    if elapsed >= RPC_SLOW {
        return ComponentHealth {
            status: HealthStatus::Degraded,
            ..health
        }
        .message(format!(
            "Stellar RPC responded in {}ms",
            elapsed.as_millis()
        ));
    }
    health
}

/// Directory whose filesystem is checked: the working directory, where the
/// service writes anything it writes.
fn disk_probe_path() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
}

/// Free space on the filesystem holding `path`.
pub fn check_disk(path: &Path) -> ComponentHealth {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    // The disk whose mount point is the longest prefix of `path`.
    let disk = disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len());

    match disk {
        Some(disk) => disk_component(
            &disk.mount_point().display().to_string(),
            disk.available_space(),
            disk.total_space(),
        ),
        None => ComponentHealth::new(HealthStatus::Degraded, false, "unknown")
            .message("could not determine disk usage"),
    }
}

fn disk_component(mount_point: &str, available: u64, total: u64) -> ComponentHealth {
    if total == 0 {
        return ComponentHealth::new(HealthStatus::Degraded, false, "unknown")
            .message(format!("disk {mount_point} reports zero capacity"));
    }
    let free_ratio = available as f64 / total as f64;
    let details = json!({
        "mount_point": mount_point,
        "available_bytes": available,
        "total_bytes": total,
        "free_ratio": (free_ratio * 1000.0).round() / 1000.0,
    });
    let low = || {
        format!(
            "low disk space on {mount_point}: {:.1}% free ({available}/{total} bytes)",
            free_ratio * 100.0
        )
    };

    if free_ratio < DISK_UNHEALTHY_FREE_RATIO {
        ComponentHealth::new(HealthStatus::Unhealthy, false, "critical")
            .message(low())
            .details(details)
    } else if free_ratio < DISK_DEGRADED_FREE_RATIO {
        ComponentHealth::new(HealthStatus::Degraded, false, "low")
            .message(low())
            .details(details)
    } else {
        ComponentHealth::new(HealthStatus::Healthy, false, "ok").details(details)
    }
}

/// The in-memory oracle price cache must have been populated at least once.
pub fn check_price_cache(cache: &PriceCache) -> ComponentHealth {
    let assets = cache.snapshot().len();
    if assets == 0 {
        ComponentHealth::new(HealthStatus::Unhealthy, true, "not_ready")
            .message("price cache is empty")
            .details(json!({ "assets": 0 }))
    } else {
        ComponentHealth::new(HealthStatus::Healthy, true, "ok").details(json!({ "assets": assets }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(status: HealthStatus) -> ComponentHealth {
        ComponentHealth::new(status, true, "ok")
    }

    #[test]
    fn overall_status_is_worst_component() {
        let start = Instant::now();
        let report = HealthReport::from_components(
            BTreeMap::from([
                ("a", component(HealthStatus::Healthy)),
                ("b", component(HealthStatus::Degraded)),
            ]),
            start,
        );
        assert_eq!(report.status, HealthStatus::Degraded);
        assert_eq!(
            report.http_status(),
            StatusCode::OK,
            "degraded still serves traffic"
        );

        let report = HealthReport::from_components(
            BTreeMap::from([
                ("a", component(HealthStatus::Degraded)),
                ("b", component(HealthStatus::Unhealthy)),
            ]),
            start,
        );
        assert_eq!(report.status, HealthStatus::Unhealthy);
        assert_eq!(report.http_status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn json_contains_components_and_legacy_fields() {
        let report = HealthReport::from_components(
            BTreeMap::from([
                ("database", check_database_unconfigured()),
                (
                    "redis",
                    ComponentHealth::new(HealthStatus::Degraded, false, "unreachable")
                        .message("Redis ping failed"),
                ),
            ]),
            Instant::now(),
        );
        let body = report.to_json("v1");
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["health"], "degraded");
        assert_eq!(body["version"], "v1");
        assert_eq!(body["components"]["redis"]["status"], "degraded");
        assert_eq!(body["components"]["redis"]["critical"], false);
        assert_eq!(body["dependencies"]["db"], "not_configured");
        assert_eq!(body["dependencies"]["redis"], "unreachable");
        assert_eq!(body["errors"]["redis"], "Redis ping failed");
        assert!(
            body["errors"]["db"].is_null(),
            "unconfigured is not an error"
        );
        assert!(body["components"]["database"].get("latency_ms").is_none());
    }

    fn check_database_unconfigured() -> ComponentHealth {
        ComponentHealth::new(HealthStatus::Healthy, true, "not_configured")
            .message("database pool is not configured")
    }

    #[tokio::test]
    async fn unconfigured_database_is_healthy_but_flagged() {
        let health = check_database(None).await;
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.legacy, "not_configured");
    }

    #[tokio::test]
    async fn redis_down_is_degraded_unless_required() {
        let optional = check_redis(&RedisCache::disabled(), false).await;
        assert_eq!(optional.status, HealthStatus::Degraded);
        assert!(!optional.critical);

        let required = check_redis(&RedisCache::disabled(), true).await;
        assert_eq!(required.status, HealthStatus::Unhealthy);
        assert!(required.critical);

        let up = check_redis(&RedisCache::simulate_available(), true).await;
        assert_eq!(up.status, HealthStatus::Healthy);
        assert!(up.latency_ms.is_some());
    }

    #[test]
    fn rpc_component_interprets_node_status() {
        let healthy = rpc_component(
            &json!({ "result": { "status": "healthy", "latestLedger": 42 } }),
            Duration::from_millis(5),
            1,
        );
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.details.as_ref().unwrap()["latest_ledger"], 42);

        let catching_up = rpc_component(
            &json!({ "result": { "status": "catching_up" } }),
            Duration::from_millis(5),
            1,
        );
        assert_eq!(catching_up.status, HealthStatus::Degraded);

        let slow = rpc_component(&json!({ "result": { "status": "healthy" } }), RPC_SLOW, 2);
        assert_eq!(slow.status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn unreachable_rpc_is_unhealthy() {
        let health = check_stellar_rpc("http://127.0.0.1:1/", Duration::from_millis(200), 1).await;
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.legacy, "unreachable");
        assert!(health
            .message
            .unwrap()
            .starts_with("Stellar RPC unreachable"));
    }

    #[tokio::test]
    async fn healthy_mock_rpc_is_healthy() {
        let mock = crate::mock_rpc_helpers::MockRpcServer::start().await;
        let health = check_stellar_rpc(&mock.url(), Duration::from_secs(2), 1).await;
        mock.shutdown().await;
        assert_eq!(health.status, HealthStatus::Healthy, "{:?}", health.message);
    }

    #[test]
    fn disk_thresholds() {
        assert_eq!(disk_component("/", 50, 100).status, HealthStatus::Healthy);
        assert_eq!(disk_component("/", 8, 100).status, HealthStatus::Degraded);
        assert_eq!(disk_component("/", 4, 100).status, HealthStatus::Unhealthy);
        assert_eq!(disk_component("/", 0, 0).status, HealthStatus::Degraded);
    }

    #[test]
    fn check_disk_picks_a_mounted_filesystem() {
        let health = check_disk(&disk_probe_path());
        // Whatever the host, the probe must produce a classified result with
        // details when the filesystem is found.
        if health.legacy != "unknown" {
            assert!(health.details.unwrap()["total_bytes"].as_u64().unwrap() > 0);
        }
    }

    #[test]
    fn price_cache_must_be_populated() {
        let health = check_price_cache(&PriceCache::new());
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.legacy, "not_ready");
    }
}
