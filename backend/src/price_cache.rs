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
    time::Duration,
};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

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

/// Shared, thread-safe price cache.
#[derive(Clone, Default)]
pub struct PriceCache(Arc<RwLock<HashMap<String, f64>>>);

impl PriceCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Overwrite the cache with a fresh snapshot.
    pub fn update(&self, prices: HashMap<String, f64>) {
        if let Ok(mut guard) = self.0.write() {
            *guard = prices;
        }
    }

    /// Read a snapshot of all cached prices.
    pub fn snapshot(&self) -> HashMap<String, f64> {
        self.0.read().map(|g| g.clone()).unwrap_or_default()
    }
}

// ── Background fetcher ───────────────────────────────────────────────────────

/// CoinGecko IDs for the assets we track.
const ASSETS: &[(&str, &str)] = &[("BTC", "bitcoin"), ("ETH", "ethereum"), ("XLM", "stellar")];

/// Spawn a background task that refreshes the cache every 60 seconds.
pub fn spawn_fetcher(cache: PriceCache) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");

        loop {
            match fetch_prices(&client).await {
                Ok(prices) => {
                    info!(assets = prices.len(), "price cache refreshed");
                    cache.update(prices);
                }
                Err(err) => {
                    error!(error = %err, "price fetch failed; retaining stale cache");
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
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
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("BTC"));
        assert!(text.contains("50000"));
    }
}
