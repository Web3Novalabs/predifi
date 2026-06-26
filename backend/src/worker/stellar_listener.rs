//! Stellar RPC event listener.
//!
//! Polls `getEvents` on the configured Stellar RPC endpoint once per ledger
//! (~5 s). The latest processed ledger sequence is stored in the `app_state`
//! table so the worker resumes from where it left off after a restart.

use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::tracing_context;

const POLL_INTERVAL_SECS: u64 = 5;
const STATE_KEY: &str = "stellar_listener_latest_ledger";

// ── RPC response types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<GetEventsResult>,
}

#[derive(Debug, Deserialize)]
struct GetEventsResult {
    events: Vec<StellarEvent>,
    #[serde(rename = "latestLedger")]
    latest_ledger: u64,
}

/// A single event returned by the Stellar RPC `getEvents` call.
#[derive(Debug, Deserialize)]
pub struct StellarEvent {
    /// Event type string, e.g. `"contract"` or `"system"`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Ledger sequence number in which this event was emitted.
    #[serde(rename = "ledger")]
    pub ledger: u64,
    /// Soroban contract address that emitted the event, if applicable.
    #[serde(rename = "contractId")]
    pub contract_id: Option<String>,
    /// Unique event identifier assigned by the RPC node.
    pub id: String,
    /// XDR-encoded topic values decoded as strings by the RPC node.
    pub topics: Option<Vec<String>>,
    /// Arbitrary JSON payload decoded from the event's XDR data field.
    pub data: Option<Value>,
}

// ── Ledger cursor persistence ─────────────────────────────────────────────────

/// Load the last processed ledger from the database.
async fn load_cursor(pool: &PgPool) -> Option<u64> {
    sqlx::query_scalar::<_, String>("SELECT value FROM app_state WHERE key = $1")
        .bind(STATE_KEY)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
}

/// Persist the latest processed ledger to the database.
async fn save_cursor(pool: &PgPool, ledger: u64) {
    let result = sqlx::query(
        "INSERT INTO app_state (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(STATE_KEY)
    .bind(ledger.to_string())
    .execute(pool)
    .await;

    if let Err(e) = result {
        warn!(error = %e, "failed to persist ledger cursor");
    }
}

// ── RPC call ──────────────────────────────────────────────────────────────────

async fn fetch_events(
    client: &reqwest::Client,
    rpc_url: &str,
    start_ledger: u64,
) -> Result<GetEventsResult, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEvents",
        "params": {
            "startLedger": start_ledger,
            "filters": []
        }
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let rpc: RpcResponse = resp.json().await.map_err(|e| e.to_string())?;
    rpc.result.ok_or_else(|| "empty RPC result".to_string())
}

// ── Worker entry point ────────────────────────────────────────────────────────

/// Spawn the Stellar event listener as a background Tokio task.
///
/// `rpc_url`   – Stellar RPC endpoint (e.g. `https://soroban-testnet.stellar.org`)
/// `db`        – PostgreSQL connection pool used to persist the ledger cursor
/// `event_bus` – broadcast channel; new predictions are published here
/// `timeout`   – maximum time to wait for an RPC response
pub fn spawn(rpc_url: String, db: PgPool, event_bus: crate::ws::EventBus, timeout: Duration) {
    tracing_context::spawn_worker("stellar_listener", async move {
        run(rpc_url, db, event_bus, timeout).await;
    });
}

async fn run(rpc_url: String, db: PgPool, event_bus: crate::ws::EventBus, timeout: Duration) {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("valid reqwest client");
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

    // Resume from the last persisted ledger, or start from ledger 1.
    let mut cursor: u64 = load_cursor(&db).await.unwrap_or(1);
    info!(cursor, "stellar listener starting");

    loop {
        ticker.tick().await;

        match fetch_events(&client, &rpc_url, cursor).await {
            Ok(result) => {
                let count = result.events.len();
                if count > 0 {
                    info!(
                        ledger_start = cursor,
                        latest_ledger = result.latest_ledger,
                        events = count,
                        "stellar events received"
                    );
                    // Collect referral events for bulk insert to minimise DB round-trips.
                    let mut referral_events: Vec<crate::db::ReferralPaidEvent> = Vec::new();

                    for event in &result.events {
                        info!(
                            id = %event.id,
                            event_type = %event.event_type,
                            ledger = event.ledger,
                            contract_id = ?event.contract_id,
                            "stellar event"
                        );

                        let topic_matches = |needle: &str| {
                            event
                                .topics
                                .as_ref()
                                .map(|t| t.iter().any(|s| s == needle))
                                .unwrap_or(false)
                        };

                        if event.event_type == "contract" {
                            if topic_matches("pool_created") {
                                if let Err(e) = handle_pool_created_event(&db, event).await {
                                    error!(
                                        id = %event.id,
                                        ledger = event.ledger,
                                        error = %e,
                                        "failed to process pool_created event"
                                    );
                                }
                            } else if topic_matches("prediction_placed") {
                                if let Err(e) =
                                    handle_prediction_placed_event(&db, event, &event_bus).await
                                {
                                    error!(
                                        id = %event.id,
                                        ledger = event.ledger,
                                        error = %e,
                                        "failed to process prediction_placed event"
                                    );
                                }
                            } else if topic_matches("pool_resolved") {
                                if let Err(e) = handle_pool_resolved_event(&db, event).await {
                                    error!(
                                        id = %event.id,
                                        ledger = event.ledger,
                                        error = %e,
                                        "failed to process pool_resolved event"
                                    );
                                }
                            } else if topic_matches("pool_canceled") {
                                if let Err(e) = handle_pool_canceled_event(&db, event).await {
                                    error!(
                                        id = %event.id,
                                        ledger = event.ledger,
                                        error = %e,
                                        "failed to process pool_canceled event"
                                    );
                                }
                            } else if topic_matches("referral_paid") {
                                match parse_referral_paid_event(event) {
                                    Ok(ev) => referral_events.push(ev),
                                    Err(e) => error!(
                                        id = %event.id,
                                        ledger = event.ledger,
                                        error = %e,
                                        "failed to parse referral_paid event"
                                    ),
                                }
                            }
                        }
                    }

                    // Bulk-insert all collected referral events in a single query.
                    if !referral_events.is_empty() {
                        if let Err(e) =
                            crate::db::insert_referrals_bulk(&db, &referral_events).await
                        {
                            error!(
                                error = %e,
                                count = referral_events.len(),
                                "failed to bulk insert referral events"
                            );
                        }
                    }
                }

                let new_cursor = result.latest_ledger + 1;
                if new_cursor > cursor {
                    cursor = new_cursor;
                    save_cursor(&db, cursor).await;
                }
            }
            Err(e) => {
                error!(error = %e, cursor, "failed to fetch stellar events");
            }
        }
    }
}

async fn handle_pool_created_event(db: &PgPool, event: &StellarEvent) -> Result<(), String> {
    let data = event
        .data
        .as_ref()
        .ok_or_else(|| "missing event data".to_string())?;

    let pool_id =
        extract_u64(data, "pool_id").ok_or_else(|| "missing or invalid pool_id".to_string())?;
    let creator =
        extract_string(data, "creator").ok_or_else(|| "missing or invalid creator".to_string())?;
    let end_time =
        extract_u64(data, "end_time").ok_or_else(|| "missing or invalid end_time".to_string())?;
    let token =
        extract_string(data, "token").ok_or_else(|| "missing or invalid token".to_string())?;
    let category = extract_string(data, "category").unwrap_or_default();
    // The on-chain event carries metadata_url; use it as the pool name/description.
    let description = extract_string(data, "description")
        .or_else(|| extract_string(data, "metadata_url"))
        .unwrap_or_default();

    let pool_event = crate::db::PoolCreatedEvent {
        pool_id,
        creator,
        end_time,
        token,
        category,
        description,
    };

    crate::db::insert_pool_from_event(db, &pool_event)
        .await
        .map_err(|e| e.to_string())
}

async fn handle_prediction_placed_event(
    db: &PgPool,
    event: &StellarEvent,
    event_bus: &crate::ws::EventBus,
) -> Result<(), String> {
    let data = event
        .data
        .as_ref()
        .ok_or_else(|| "missing event data".to_string())?;

    let pool_id = extract_u64(data, "pool_id")
        .ok_or_else(|| "missing or invalid pool_id in event data".to_string())?;
    let user_address = extract_string(data, "user")
        .or_else(|| extract_string(data, "user_address"))
        .ok_or_else(|| "missing or invalid user address in event data".to_string())?;
    let amount = extract_i64(data, "amount")
        .ok_or_else(|| "missing or invalid amount in event data".to_string())?;
    let outcome = extract_i32(data, "outcome")
        .ok_or_else(|| "missing or invalid outcome in event data".to_string())?;

    let ev = crate::db::PredictionPlacedEvent {
        pool_id,
        user_address,
        outcome,
        amount,
    };

    crate::db::insert_prediction_from_event_with_pool(db, &ev)
        .await
        .map_err(|e| e.to_string())?;

    event_bus.send(&serde_json::json!({
        "type": "prediction_placed",
        "pool_id": ev.pool_id,
        "user_address": ev.user_address,
        "outcome": ev.outcome,
        "amount": ev.amount,
    }));

    Ok(())
}

async fn handle_pool_resolved_event(db: &PgPool, event: &StellarEvent) -> Result<(), String> {
    let data = event
        .data
        .as_ref()
        .ok_or_else(|| "missing event data".to_string())?;

    let pool_id =
        extract_u64(data, "pool_id").ok_or_else(|| "missing or invalid pool_id".to_string())?;
    let outcome =
        extract_i32(data, "outcome").ok_or_else(|| "missing or invalid outcome".to_string())?;

    crate::db::resolve_pool_in_db(db, pool_id, outcome)
        .await
        .map_err(|e| e.to_string())
}

async fn handle_pool_canceled_event(db: &PgPool, event: &StellarEvent) -> Result<(), String> {
    let data = event
        .data
        .as_ref()
        .ok_or_else(|| "missing event data".to_string())?;

    let pool_id =
        extract_u64(data, "pool_id").ok_or_else(|| "missing or invalid pool_id".to_string())?;

    crate::db::cancel_pool_in_db(db, pool_id)
        .await
        .map_err(|e| e.to_string())
}

/// Parse a `referral_paid` event into a [`ReferralPaidEvent`] without touching the database.
///
/// This is used in conjunction with `insert_referrals_bulk` so that multiple referral
/// events from a single poll cycle are inserted in one batch.
fn parse_referral_paid_event(event: &StellarEvent) -> Result<crate::db::ReferralPaidEvent, String> {
    let data = event
        .data
        .as_ref()
        .ok_or_else(|| "missing event data".to_string())?;

    let pool_id =
        extract_u64(data, "pool_id").ok_or_else(|| "missing or invalid pool_id".to_string())?;
    let referrer = extract_string(data, "referrer")
        .ok_or_else(|| "missing or invalid referrer".to_string())?;
    let referred_user = extract_string(data, "referred_user")
        .or_else(|| extract_string(data, "user"))
        .ok_or_else(|| "missing or invalid referred_user".to_string())?;
    let referral_amount = extract_i64(data, "referral_amount")
        .or_else(|| extract_i64(data, "amount"))
        .ok_or_else(|| "missing or invalid referral_amount".to_string())?;

    Ok(crate::db::ReferralPaidEvent {
        pool_id,
        referrer,
        referred_user,
        referral_amount,
    })
}

fn extract_string(data: &Value, key: &str) -> Option<String> {
    let value = data.get(key)?;
    extract_string_value(value)
}

fn extract_string_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Object(map) if map.len() == 1 => map.values().next().and_then(extract_string_value),
        _ => None,
    }
}

fn extract_i128(value: &Value) -> Option<i128> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(|v| v as i128)
            .or_else(|| number.as_u64().map(i128::from)),
        Value::String(s) => s.parse().ok(),
        Value::Object(map) if map.len() == 1 => map.values().next().and_then(extract_i128),
        _ => None,
    }
}

fn extract_i64(data: &Value, key: &str) -> Option<i64> {
    extract_i128(data.get(key)?).and_then(|v| i64::try_from(v).ok())
}

fn extract_i32(data: &Value, key: &str) -> Option<i32> {
    extract_i128(data.get(key)?).and_then(|v| i32::try_from(v).ok())
}

fn extract_u64(data: &Value, key: &str) -> Option<u64> {
    extract_i128(data.get(key)?).and_then(|v| u64::try_from(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rpc_response_with_events() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "events": [
                    {
                        "type": "contract",
                        "ledger": 42,
                        "contractId": "CABC123",
                        "id": "evt-1"
                    }
                ],
                "latestLedger": 42
            }
        }"#;

        let resp: RpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result.latest_ledger, 42);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].id, "evt-1");
    }

    #[test]
    fn parse_rpc_response_empty_events() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "events": [],
                "latestLedger": 100
            }
        }"#;

        let resp: RpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result.latest_ledger, 100);
        assert!(result.events.is_empty());
    }

    /// Verify that pool_created event data is parsed into the correct fields.
    #[test]
    fn extract_pool_created_fields_from_event_data() {
        let data = serde_json::json!({
            "pool_id": 7,
            "creator": "GABC123",
            "end_time": 1_700_000_000u64,
            "token": "GTOKEN",
            "category": "Sports",
            "metadata_url": "ipfs://Qm123"
        });

        assert_eq!(extract_u64(&data, "pool_id"), Some(7));
        assert_eq!(extract_string(&data, "creator"), Some("GABC123".into()));
        assert_eq!(extract_u64(&data, "end_time"), Some(1_700_000_000));
        assert_eq!(extract_string(&data, "token"), Some("GTOKEN".into()));
        assert_eq!(extract_string(&data, "category"), Some("Sports".into()));
        // description absent → falls back to metadata_url
        assert_eq!(extract_string(&data, "description"), None);
        assert_eq!(
            extract_string(&data, "metadata_url"),
            Some("ipfs://Qm123".into())
        );
    }

    /// Missing required fields must produce None so the handler returns an error.
    #[test]
    fn extract_pool_created_missing_required_field_returns_none() {
        let data = serde_json::json!({ "pool_id": 1 });
        assert!(extract_string(&data, "creator").is_none());
        assert!(extract_u64(&data, "end_time").is_none());
    }
}
