//! # Oracle Price Cache Service
//!
//! Fetches current prices for BTC, ETH, and XLM from CoinGecko every 60 seconds
//! and stores them in a shared in-memory cache so the frontend can read them
//! without hitting the blockchain or an external API on every request.
//!
//! ## Usage
//!
//! ```rust,ignore
//! // In main / router setup:
//! let cache = price_cache::PriceCache::new();
//! price_cache::spawn_fetcher(cache.clone(), Some(metrics.clone()));
//!
//! // In the Axum router (state must also expose `SharedMetrics` via
//! // `FromRef`, since `get_prices` uses it for on-demand refresh metrics):
//! Router::new()
//!     .route("/api/v1/prices", get(price_cache::get_prices))
//!     .with_state(app_state)
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{error, info, info_span, Instrument};

use crate::metrics::SharedMetrics;
use crate::response::ApiResponse;

// ── Types ────────────────────────────────────────────────────────────────────

/// A single asset price entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPrice {
    /// Asset symbol, e.g. `"BTC"`.
    pub symbol: String,
    /// Price in USD.
    pub price_usd: f64,
}

/// How long a cached price snapshot is considered fresh before an on-demand
/// reader should attempt to refresh it, rather than waiting for the next
/// scheduled [`spawn_fetcher`] tick. Matches the background refresh interval.
const PRICE_CACHE_TTL: Duration = Duration::from_secs(60);

/// Inner, `Arc`-shared state for [`PriceCache`].
struct Inner {
    prices: RwLock<HashMap<String, f64>>,
    /// When the cache was last successfully populated, used to decide
    /// whether an on-demand refresh (see [`PriceCache::ensure_fresh`]) is due.
    last_updated: RwLock<Option<Instant>>,
    /// Single-flight guard: whoever holds this lock is the one request that
    /// performs the upstream CoinGecko fetch. Every other concurrent caller
    /// blocks on the same lock and, once it acquires it, re-checks freshness
    /// and finds the cache already refreshed — so a burst of concurrent
    /// requests for an expired cache results in exactly one upstream fetch.
    refresh_lock: tokio::sync::Mutex<()>,
    /// HTTP client shared between the background fetcher and any on-demand
    /// refresh, so both paths reuse the same connection pool.
    client: reqwest::Client,
}

/// Shared, thread-safe price cache.
#[derive(Clone)]
pub struct PriceCache(Arc<Inner>);

impl Default for PriceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceCache {
    /// Create an empty price cache.
    ///
    /// The cache starts with no entries.  Call [`update`](Self::update) to
    /// populate it, or rely on [`spawn_fetcher`] to refresh it periodically
    /// from CoinGecko.
    ///
    /// # Panics
    ///
    /// Panics if the reqwest HTTP client cannot be built (this should never
    /// happen in practice).
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self(Arc::new(Inner {
            prices: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(None),
            refresh_lock: tokio::sync::Mutex::new(()),
            client,
        }))
    }

    /// Overwrite the cache with a fresh snapshot from an external source.
    ///
    /// This acquires a write lock on the internal [`RwLock`], so it is safe to
    /// call from any thread (e.g. from a background fetcher task).  The
    /// previous contents are discarded, and the freshness clock used by
    /// [`ensure_fresh`](Self::ensure_fresh) is reset.
    ///
    /// # Panics
    ///
    /// Does not panic.  If the lock is poisoned the update is silently skipped.
    pub fn update(&self, prices: HashMap<String, f64>) {
        if let Ok(mut guard) = self.0.prices.write() {
            *guard = prices;
        }
        if let Ok(mut guard) = self.0.last_updated.write() {
            *guard = Some(Instant::now());
        }
    }

    /// Return a copy of all currently cached prices.
    ///
    /// Acquires a read lock on the internal [`RwLock`], so the returned map
    /// reflects a consistent point-in-time view.  If the cache has never been
    /// populated the map will be empty.
    ///
    /// # Panics
    ///
    /// Does not panic.  If the lock is poisoned an empty map is returned.
    pub fn snapshot(&self) -> HashMap<String, f64> {
        self.0.prices.read().map(|g| g.clone()).unwrap_or_default()
    }

    /// `true` only if the cache was previously populated at least once but
    /// the entry is now older than [`PRICE_CACHE_TTL`] — i.e. it has
    /// *expired*, as opposed to a cold cache that has never been populated
    /// (`last_updated` is `None`). A cold cache is left to the background
    /// [`spawn_fetcher`] rather than triggering an on-demand fetch here, so
    /// behaviour at startup (before the first successful fetch) is unchanged:
    /// callers keep seeing an empty snapshot until that first fetch lands.
    fn is_expired(&self) -> bool {
        match self.0.last_updated.read() {
            Ok(guard) => matches!(*guard, Some(last) if last.elapsed() >= PRICE_CACHE_TTL),
            Err(_) => false,
        }
    }

    /// Return a fresh snapshot, refreshing from CoinGecko on demand if a
    /// previously-cached entry has expired — with cache-stampede protection.
    ///
    /// The background [`spawn_fetcher`] task normally keeps the cache warm
    /// every [`PRICE_CACHE_TTL`], so this only does real work if that
    /// background task has stalled and an entry has gone stale. (A cold
    /// cache that has never been populated is left alone here — see
    /// [`is_expired`](Self::is_expired) — so cold-start behaviour is
    /// unchanged.) When a refresh is needed, concurrent callers are
    /// coalesced via an internal single-flight lock: the first caller
    /// performs the one upstream fetch, and every other caller racing on the
    /// same expired entry blocks on that lock, then re-checks freshness and
    /// returns the value the first caller just fetched — so a burst of
    /// concurrent requests for an expired entry never causes more than one
    /// upstream call. If the fetch fails, the previous (stale) snapshot is
    /// returned rather than an error.
    pub async fn ensure_fresh(&self, metrics: Option<&SharedMetrics>) -> HashMap<String, f64> {
        if !self.is_expired() {
            return self.snapshot();
        }

        let _guard = self.0.refresh_lock.lock().await;

        // Re-check after acquiring the lock: another caller may have already
        // refreshed the cache while we were waiting for it.
        if !self.is_expired() {
            return self.snapshot();
        }

        let fetch_started = Instant::now();
        match fetch_prices(&self.0.client).await {
            Ok(prices) => {
                let asset_count = prices.len();
                let duration_secs = fetch_started.elapsed().as_secs_f64();
                info!(
                    assets = asset_count,
                    duration_secs, "price cache refreshed (on-demand)"
                );
                self.update(prices);
                if let Some(metrics) = metrics {
                    metrics.record_price_cache_fetch("success", asset_count, duration_secs);
                }
            }
            Err(err) => {
                let duration_secs = fetch_started.elapsed().as_secs_f64();
                let cached_assets = self.snapshot().len();
                error!(
                    error = %err,
                    duration_secs, "on-demand price fetch failed; retaining stale cache"
                );
                if let Some(metrics) = metrics {
                    metrics.record_price_cache_fetch("failure", cached_assets, duration_secs);
                }
            }
        }

        self.snapshot()
    }
}

// ── Background fetcher ───────────────────────────────────────────────────────

/// CoinGecko IDs for the assets we track.
const ASSETS: &[(&str, &str)] = &[("BTC", "bitcoin"), ("ETH", "ethereum"), ("XLM", "stellar")];

/// Spawn a background Tokio task that refreshes the cache from CoinGecko
/// every 60 seconds.
///
/// On success the cache is atomically overwritten with the latest prices.
/// On failure (network error, rate limit, etc.) the previous data is
/// retained and the error is logged — the cache never goes backwards.
///
/// The returned [`JoinHandle`] allows the caller (typically the graceful
/// shutdown sequence in [`crate::server`]) to abort the fetcher task when
/// the process is winding down so it does not keep the runtime alive
/// after the HTTP listener has stopped.
///
/// Shares the same HTTP client and single-flight refresh lock as
/// [`PriceCache::ensure_fresh`], so a scheduled tick here can never race
/// with an on-demand refresh triggered by a request handler.
pub fn spawn_fetcher(cache: PriceCache, metrics: Option<SharedMetrics>) -> JoinHandle<()> {
    crate::tracing_context::spawn_worker("price_cache_fetcher", async move {
        loop {
            let span = info_span!("price_cache.fetch");
            let fetch_started = Instant::now();

            let fetch_result = async {
                let _guard = cache.0.refresh_lock.lock().await;
                fetch_prices(&cache.0.client).await
            }
            .instrument(span)
            .await;

            let duration_secs = fetch_started.elapsed().as_secs_f64();

            match fetch_result {
                Ok(prices) => {
                    let asset_count = prices.len();
                    info!(assets = asset_count, duration_secs, "price cache refreshed");
                    cache.update(prices);
                    if let Some(ref metrics) = metrics {
                        metrics.record_price_cache_fetch("success", asset_count, duration_secs);
                    }
                }
                Err(err) => {
                    let cached_assets = cache.snapshot().len();
                    error!(error = %err, duration_secs, "price fetch failed; retaining stale cache");
                    if let Some(ref metrics) = metrics {
                        metrics.record_price_cache_fetch("failure", cached_assets, duration_secs);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}

/// Fetch prices from CoinGecko simple/price endpoint.
async fn fetch_prices(client: &reqwest::Client) -> Result<HashMap<String, f64>, reqwest::Error> {
    let ids: Vec<&str> = ASSETS.iter().map(|(_, id)| *id).collect();
    let ids_param = ids.join(",");

    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
        ids_param
    );

    // Response shape: { "bitcoin": { "usd": 60000.0 }, ... }
    let raw: HashMap<String, HashMap<String, f64>> = client.get(&url).send().await?.json().await?;

    let mut result = HashMap::new();
    for (symbol, coingecko_id) in ASSETS {
        if let Some(inner) = raw.get(*coingecko_id) {
            if let Some(&price) = inner.get("usd") {
                result.insert(symbol.to_string(), price);
            }
        }
    }
    Ok(result)
}

// ── HTTP handler ─────────────────────────────────────────────────────────────

/// `GET /api/v1/prices`
///
/// Returns the latest cached prices for BTC, ETH, and XLM, refreshing
/// on-demand (with cache-stampede protection, see
/// [`PriceCache::ensure_fresh`]) if the background fetcher hasn't populated
/// the cache yet or it has gone stale.
/// Responds with 503 only if a refresh was attempted and the cache is still
/// empty (e.g. CoinGecko is unreachable on cold start).
pub async fn get_prices(
    State(cache): State<PriceCache>,
    State(metrics): State<SharedMetrics>,
) -> (StatusCode, Json<ApiResponse<Vec<AssetPrice>>>) {
    use crate::response::error_codes;
    let snapshot = cache.ensure_fresh(Some(&metrics)).await;
    if snapshot.is_empty() {
        return ApiResponse::error(
            StatusCode::SERVICE_UNAVAILABLE,
            error_codes::SERVICE_UNAVAILABLE,
            "price cache not ready",
        );
    }

    let mut prices: Vec<AssetPrice> = snapshot
        .into_iter()
        .map(|(symbol, price_usd)| AssetPrice { symbol, price_usd })
        .collect();
    // Stable ordering for deterministic responses
    prices.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    ApiResponse::success(prices)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal Axum state carrying both extractors `get_prices` needs, for
    /// tests that exercise the handler directly without the full `AppState`.
    #[derive(Clone)]
    struct TestState {
        cache: PriceCache,
        metrics: SharedMetrics,
    }

    impl axum::extract::FromRef<TestState> for PriceCache {
        fn from_ref(state: &TestState) -> Self {
            state.cache.clone()
        }
    }

    impl axum::extract::FromRef<TestState> for SharedMetrics {
        fn from_ref(state: &TestState) -> Self {
            state.metrics.clone()
        }
    }

    fn test_metrics() -> SharedMetrics {
        Arc::new(crate::metrics::Metrics::new().expect("metrics"))
    }

    #[test]
    fn cache_starts_empty() {
        let cache = PriceCache::new();
        assert!(cache.snapshot().is_empty());
    }

    #[test]
    fn cache_update_and_read() {
        let cache = PriceCache::new();
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), 60_000.0);
        prices.insert("ETH".to_string(), 3_000.0);
        cache.update(prices);

        let snap = cache.snapshot();
        assert_eq!(snap.get("BTC"), Some(&60_000.0));
        assert_eq!(snap.get("ETH"), Some(&3_000.0));
    }

    #[test]
    fn cache_clone_shares_state() {
        let cache = PriceCache::new();
        let clone = cache.clone();

        let mut prices = HashMap::new();
        prices.insert("XLM".to_string(), 0.12);
        cache.update(prices);

        assert_eq!(clone.snapshot().get("XLM"), Some(&0.12));
    }

    #[tokio::test]
    async fn get_prices_returns_503_when_empty() {
        use axum::{body::Body, http::Request};
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let cache = PriceCache::new();
        let app = axum::Router::new()
            .route("/prices", axum::routing::get(get_prices))
            .with_state(TestState {
                cache,
                metrics: test_metrics(),
            });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/prices")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        // Consume the body so the underlying stream is closed before the test exits.
        let _ = response.into_body().collect().await.unwrap();
    }

    #[tokio::test]
    async fn get_prices_returns_200_when_populated() {
        use axum::{body::Body, http::Request};
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let cache = PriceCache::new();
        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), 50_000.0);
        cache.update(prices);

        let app = axum::Router::new()
            .route("/prices", axum::routing::get(get_prices))
            .with_state(TestState {
                cache,
                metrics: test_metrics(),
            });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/prices")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        // Consume the full body before the test exits.
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("BTC"));
        assert!(text.contains("50000"));
    }

    #[test]
    fn metrics_record_price_cache_fetch() {
        let metrics = crate::metrics::Metrics::new().expect("metrics");
        metrics.record_price_cache_fetch("success", 3, 0.25);
        let text = metrics.gather_text().expect("metrics text");
        assert!(text.contains("app_price_cache_fetch_total"));
        assert!(text.contains("app_price_cache_assets"));
        assert!(text.contains("app_price_cache_fetch_duration_seconds"));
    }
}
