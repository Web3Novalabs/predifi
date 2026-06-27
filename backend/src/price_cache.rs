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
//! price_cache::spawn_fetcher(cache.clone());
//!
//! // In the Axum router:
//! Router::new()
//!     .route("/api/v1/prices", get(price_cache::get_prices))
//!     .with_state(cache)
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
use crate::tracing_context;

// ── Types ────────────────────────────────────────────────────────────────────

/// A single asset price entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPrice {
    /// Asset symbol, e.g. `"BTC"`.
    pub symbol: String,
    /// Price in USD.
    pub price_usd: f64,
}

/// Shared, thread-safe price cache.
#[derive(Clone, Default)]
pub struct PriceCache(Arc<RwLock<HashMap<String, f64>>>);

impl PriceCache {
    /// Create an empty price cache.
    ///
    /// The cache starts with no entries.  Call [`update`](Self::update) to
    /// populate it, or rely on [`spawn_fetcher`] to refresh it periodically
    /// from CoinGecko.
    pub fn new() -> Self {
        Self::default()
    }

    /// Overwrite the cache with a fresh snapshot from an external source.
    ///
    /// This acquires a write lock on the internal [`RwLock`], so it is safe to
    /// call from any thread (e.g. from a background fetcher task).  The
    /// previous contents are discarded.
    ///
    /// # Panics
    ///
    /// Does not panic.  If the lock is poisoned the update is silently skipped.
    pub fn update(&self, prices: HashMap<String, f64>) {
        if let Ok(mut guard) = self.0.write() {
            *guard = prices;
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
        self.0.read().map(|g| g.clone()).unwrap_or_default()
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
/// # Panics
///
/// Panics if the reqwest HTTP client cannot be built (this should never
/// happen in practice).
pub fn spawn_fetcher(cache: PriceCache, metrics: Option<SharedMetrics>) -> JoinHandle<()> {
    tracing_context::spawn_worker("price_cache_fetcher", async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");

        loop {
            let span = info_span!("price_cache.fetch");
            let fetch_started = Instant::now();

            let fetch_result = async {
                fetch_prices(&client).await
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

/// Fetch prices from CoinCap as a fallback.
async fn fetch_prices_fallback(client: &reqwest::Client) -> Result<HashMap<String, f64>, reqwest::Error> {
    let ids: Vec<&str> = ASSETS.iter().map(|(_, id)| *id).collect();
    let ids_param = ids.join(",");

    let url = format!(
        "https://api.coincap.io/v2/assets?ids={}",
        ids_param
    );

    #[derive(serde::Deserialize)]
    struct CoinCapAsset {
        symbol: String,
        #[serde(rename = "priceUsd")]
        price_usd: String,
    }

    #[derive(serde::Deserialize)]
    struct CoinCapResponse {
        data: Vec<CoinCapAsset>,
    }

    let raw: CoinCapResponse = client.get(&url).send().await?.json().await?;

    let mut result = HashMap::new();
    for asset in raw.data {
        if let Ok(price) = asset.price_usd.parse::<f64>() {
            if ASSETS.iter().any(|(s, _)| *s == asset.symbol) {
                result.insert(asset.symbol, price);
            }
        }
    }
    Ok(result)
}

// ── HTTP handler ─────────────────────────────────────────────────────────────

/// `GET /api/v1/prices`
///
/// Returns the latest cached prices for BTC, ETH, and XLM.
/// Responds with 503 if the cache has not been populated yet.
pub async fn get_prices(
    State(cache): State<PriceCache>,
) -> (StatusCode, Json<ApiResponse<Vec<AssetPrice>>>) {
    let snapshot = cache.snapshot();
    if snapshot.is_empty() {
        return ApiResponse::error(StatusCode::SERVICE_UNAVAILABLE, "price cache not ready");
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
            .with_state(cache);

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
            .with_state(cache);

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
