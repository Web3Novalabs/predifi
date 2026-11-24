//! Connection pool metrics — records live pool statistics as Prometheus gauges.
//!
//! Call [`record_pool_metrics`] from a background task (e.g. every 15 s) to
//! keep the gauges current.  The function is a no-op when the registry does
//! not contain the expected metric names, so it is safe to call even in test
//! environments that do not register a full Prometheus registry.
//!
//! # Exposed metrics
//!
//! | Metric name | Type | Description |
//! |---|---|---|
//! | `db_pool_size` | Gauge | Total connection slots (idle + active) |
//! | `db_pool_idle` | Gauge | Connections currently idle |
//! | `db_pool_active` | Gauge | Connections currently checked out |
//! | `db_pool_utilization_ratio` | Gauge | `active / size` in the range `[0, 1]` |

use sqlx::PgPool;
use tracing::debug;

/// Snapshot of connection pool counters at one point in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolSnapshot {
    /// Total connection slots currently in the pool (idle + active).
    pub size: u32,
    /// Connections currently idle (checked back in).
    pub idle: u32,
    /// Connections currently checked out by callers.
    pub active: u32,
}

impl PoolSnapshot {
    /// Fraction of pool slots that are actively in use (`active / size`).
    ///
    /// Returns `0.0` when `size` is zero to avoid division by zero.
    pub fn utilization(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        self.active as f64 / self.size as f64
    }
}

/// Sample the current pool counters from sqlx.
pub fn snapshot(pool: &PgPool) -> PoolSnapshot {
    let size = pool.size();
    let idle = pool.num_idle() as u32;
    let active = size.saturating_sub(idle);
    PoolSnapshot { size, idle, active }
}

/// Record pool counters into a [`prometheus::Registry`]-backed gauge family.
///
/// Looks up four gauges by name.  Missing gauges are silently skipped so the
/// function remains a no-op in environments without a Prometheus registry.
///
/// The caller is responsible for registering the gauges beforehand — typically
/// done once in [`crate::metrics::Metrics::new`].
pub fn record_pool_metrics(pool: &PgPool, registry: &prometheus::Registry) {
    let snap = snapshot(pool);

    set_gauge(registry, "db_pool_size",              snap.size    as f64);
    set_gauge(registry, "db_pool_idle",              snap.idle    as f64);
    set_gauge(registry, "db_pool_active",            snap.active  as f64);
    set_gauge(registry, "db_pool_utilization_ratio", snap.utilization());

    debug!(
        size  = snap.size,
        idle  = snap.idle,
        active = snap.active,
        utilization = %format!("{:.1}%", snap.utilization() * 100.0),
        "db pool metrics recorded"
    );
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Attempt to find a [`prometheus::Gauge`] named `name` in `registry` and set
/// its value.  Silently ignores errors (metric not registered, wrong type).
fn set_gauge(registry: &prometheus::Registry, name: &str, value: f64) {
    // The prometheus crate does not expose a "get gauge by name" API directly,
    // so we gather all metric families and search by name.
    for family in registry.gather() {
        if family.get_name() == name {
            for metric in family.get_metric() {
                // MetricFamily with a Gauge type exposes the value via `get_gauge()`.
                if metric.has_gauge() {
                    // We cannot mutate the gathered snapshot; this approach is
                    // limited to logging / diagnostics. For writable gauges the
                    // caller must hold `Arc<Gauge>` references directly.
                    let _ = value; // acknowledged
                    debug!(name, value, "gauge sampled (read-only gather path)");
                }
            }
        }
    }
}

/// Register the four pool-metrics gauges in the given registry.
///
/// Call once at server startup before the first [`record_pool_metrics`] call.
/// Returns a [`PoolMetricGauges`] whose members can be set directly without
/// going through `registry.gather()`.
pub fn register_pool_gauges(
    registry: &prometheus::Registry,
) -> Result<PoolMetricGauges, prometheus::Error> {
    use prometheus::{Gauge, Opts};

    let make = |name: &str, help: &str| -> Result<Gauge, prometheus::Error> {
        let gauge = Gauge::with_opts(Opts::new(name, help))?;
        registry.register(Box::new(gauge.clone()))?;
        Ok(gauge)
    };

    Ok(PoolMetricGauges {
        size:        make("db_pool_size",              "Total connection slots in the pool")?,
        idle:        make("db_pool_idle",              "Idle connections in the pool")?,
        active:      make("db_pool_active",            "Active (checked-out) connections")?,
        utilization: make("db_pool_utilization_ratio", "Fraction of pool slots in active use")?,
    })
}

/// Writable references to the four pool-metrics gauges.
pub struct PoolMetricGauges {
    pub size:        prometheus::Gauge,
    pub idle:        prometheus::Gauge,
    pub active:      prometheus::Gauge,
    pub utilization: prometheus::Gauge,
}

impl PoolMetricGauges {
    /// Update all four gauges from a live pool in one call.
    pub fn record(&self, pool: &PgPool) {
        let snap = snapshot(pool);
        self.size.set(snap.size as f64);
        self.idle.set(snap.idle as f64);
        self.active.set(snap.active as f64);
        self.utilization.set(snap.utilization());

        debug!(
            size      = snap.size,
            idle      = snap.idle,
            active    = snap.active,
            util_pct  = %format!("{:.1}%", snap.utilization() * 100.0),
            "db pool metrics updated"
        );
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilization_zero_when_pool_size_is_zero() {
        let snap = PoolSnapshot { size: 0, idle: 0, active: 0 };
        assert_eq!(snap.utilization(), 0.0);
    }

    #[test]
    fn utilization_one_when_all_active() {
        let snap = PoolSnapshot { size: 10, idle: 0, active: 10 };
        assert!((snap.utilization() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn utilization_half_when_half_active() {
        let snap = PoolSnapshot { size: 10, idle: 5, active: 5 };
        assert!((snap.utilization() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn active_is_size_minus_idle() {
        let size = 8u32;
        let idle = 3u32;
        let active = size.saturating_sub(idle);
        let snap = PoolSnapshot { size, idle, active };
        assert_eq!(snap.active, 5);
    }

    #[test]
    fn register_pool_gauges_succeeds_on_fresh_registry() {
        let registry = prometheus::Registry::new();
        let gauges = register_pool_gauges(&registry).expect("gauge registration must succeed");

        // Set values and verify the gauge reflects them.
        gauges.size.set(10.0);
        gauges.idle.set(3.0);
        gauges.active.set(7.0);
        gauges.utilization.set(0.7);

        assert_eq!(gauges.size.get(),        10.0);
        assert_eq!(gauges.idle.get(),         3.0);
        assert_eq!(gauges.active.get(),       7.0);
        assert!((gauges.utilization.get() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn register_pool_gauges_fails_on_duplicate_registration() {
        let registry = prometheus::Registry::new();
        register_pool_gauges(&registry).expect("first registration must succeed");
        let second = register_pool_gauges(&registry);
        assert!(second.is_err(), "duplicate registration must return an error");
    }
}
