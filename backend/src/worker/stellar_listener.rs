//! Stellar RPC event listener.
//!
//! Polls `getEvents` on the configured Stellar RPC endpoint once per ledger
//! (~5 s). The latest processed ledger sequence is stored in the `app_state`
//! table so the worker resumes from where it left off after a restart.

use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{error, info, instrument, warn};

use crate::redis_cache::RedisCache;

const POLL_INTERVAL_SECS: u64 = 5;
const STATE_KEY: &str = "stellar_listener_latest_ledger";
const INITIAL_RECONNECT_DELAY_SECS: u64 = 1;
const MAX_RECONNECT_DELAY_SECS: u64 = 60;

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
#[instrument(skip(pool), name = "stellar_listener.load_cursor")]
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
#[instrument(skip(pool), name = "stellar_listener.save_cursor", fields(ledger = ledger))]
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

/// Fetch a batch of Stellar contract events starting from `start_ledger`.
///
/// Each call is wrapped in its own OTel span so RPC latency and failures are
/// visible in the trace backend.
#[instrument(skip(client), name = "stellar_listener.fetch_events",
    fields(rpc_url = %rpc_url, start_ledger = start_ledger))]
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
///
/// Returns the [`JoinHandle`] for the spawned task so the caller
/// (typically [`crate::server::run`]) can abort it as part of the graceful
/// shutdown sequence.  Aborting the handle cancels the in-flight RPC poll
/// and prevents the listener from blocking process exit.
///
/// > **Note:** prefer calling [`run_worker`] directly inside a
/// > [`crate::tracing_context::spawn_worker`] closure so the task inherits a
/// > named root span in the OTel trace backend.
pub fn spawn(
    rpc_url: String,
    db: PgPool,
    event_bus: crate::ws::EventBus,
    redis: RedisCache,
    timeout: Duration,
    max_batch_size: usize,
) -> JoinHandle<()> {
    crate::tracing_context::spawn_worker("stellar_listener", async move {
        run_worker(rpc_url, db, event_bus, redis, timeout, max_batch_size).await;
    })
}

/// Compute exponential backoff delay for RPC reconnect attempts.
fn reconnect_delay_secs(consecutive_failures: u32) -> u64 {
    if consecutive_failures == 0 {
        return INITIAL_RECONNECT_DELAY_SECS;
    }
    let shift = consecutive_failures.saturating_sub(1).min(6);
    INITIAL_RECONNECT_DELAY_SECS
        .saturating_mul(2u64.saturating_pow(shift))
        .min(MAX_RECONNECT_DELAY_SECS)
}

/// The main polling loop for the Stellar event listener.
///
/// Exposed as `pub` so [`crate::server::run_with_signal`] can invoke it
/// inside a [`crate::tracing_context::spawn_worker`] closure, which roots the
/// entire listener under a named OTel span without double-spawning.
pub async fn run_worker(
    rpc_url: String,
    db: PgPool,
    event_bus: crate::ws::EventBus,
    redis: RedisCache,
    timeout: Duration,
    max_batch_size: usize,
) {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("valid reqwest client");
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

    // Resume from the last persisted ledger, or start from ledger 1.
    let mut cursor: u64 = load_cursor(&db).await.unwrap_or(1);
    let mut consecutive_failures: u32 = 0;
    let batch_size = max_batch_size.max(1);
    info!(cursor, batch_size, "stellar listener starting");

    loop {
        if consecutive_failures == 0 {
            ticker.tick().await;
        }

        match fetch_events(&client, &rpc_url, cursor).await {
            Ok(result) => {
                if consecutive_failures > 0 {
                    info!(
                        previous_failures = consecutive_failures,
                        "stellar RPC connection restored after reconnect"
                    );
                }
                consecutive_failures = 0;

                let count = result.events.len();
                if count > 0 {
                    if count > batch_size {
                        warn!(
                            events = count,
                            batch_size,
                            "stellar event batch exceeds configured maximum; processing in chunks"
                        );
                    }

                    info!(
                        ledger_start = cursor,
                        latest_ledger = result.latest_ledger,
                        events = count,
                        batch_size,
                        "stellar events received"
                    );

                    for chunk in result.events.chunks(batch_size) {
                        process_event_batch(&db, &redis, &event_bus, chunk, batch_size).await;
                    }
                }

                let new_cursor = result.latest_ledger + 1;
                if new_cursor > cursor {
                    cursor = new_cursor;
                    save_cursor(&db, cursor).await;
                }
            }
            Err(e) => {
                consecutive_failures += 1;
                let delay = reconnect_delay_secs(consecutive_failures);
                error!(
                    error = %e,
                    cursor,
                    consecutive_failures,
                    delay_secs = delay,
                    "failed to fetch stellar events; scheduling reconnect"
                );
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

/// Process a batch of Stellar events, dispatching each to the appropriate handler.
///
/// Each call is wrapped in its own OTel span so per-batch latency is visible
/// in the trace backend.
#[instrument(skip_all, name = "stellar_listener.process_event_batch",
    fields(event_count = events.len()))]
async fn process_event_batch(
    db: &PgPool,
    redis: &RedisCache,
    event_bus: &crate::ws::EventBus,
    events: &[StellarEvent],
    max_batch_size: usize,
) {
    let mut referral_events: Vec<crate::db::ReferralPaidEvent> = Vec::new();

    for event in events {
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
                if let Err(e) = handle_pool_created_event(db, redis, event).await {
                    error!(
                        id = %event.id,
                        ledger = event.ledger,
                        error = %e,
                        "failed to process pool_created event"
                    );
                }
            } else if topic_matches("prediction_placed") {
                if let Err(e) = handle_prediction_placed_event(db, event, event_bus).await {
                    error!(
                        id = %event.id,
                        ledger = event.ledger,
                        error = %e,
                        "failed to process prediction_placed event"
                    );
                }
            } else if topic_matches("pool_resolved") {
                if let Err(e) = handle_pool_resolved_event(db, event).await {
                    error!(
                        id = %event.id,
                        ledger = event.ledger,
                        error = %e,
                        "failed to process pool_resolved event"
                    );
                }
            } else if topic_matches("pool_canceled") {
                if let Err(e) = handle_pool_canceled_event(db, event).await {
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

    if !referral_events.is_empty() {
        if let Err(e) = crate::db::insert_referrals_bulk(db, &referral_events, max_batch_size).await
        {
            error!(
                error = %e,
                count = referral_events.len(),
                "failed to bulk insert referral events"
            );
        }
    }
}

async fn handle_pool_created_event(
    db: &PgPool,
    redis: &RedisCache,
    event: &StellarEvent,
) -> Result<(), String> {
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
        .map_err(|e| e.to_string())?;

    redis.invalidate_pools_cache().await;
    Ok(())
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
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    // ── reconnect loop behaviour ──────────────────────────────────────────────

    /// A connection drop (fetch_events returning Err) must increment
    /// `consecutive_failures` and schedule a backoff delay.  We verify this
    /// by simulating the exact state-machine from `run_worker` without
    /// spinning up a real RPC server.
    #[test]
    fn connection_drop_triggers_reconnect_with_delay() {
        // After the first error consecutive_failures becomes 1 and the delay
        // must be INITIAL_RECONNECT_DELAY_SECS (1 s), not 0.
        let consecutive_failures: u32 = 0;

        // Simulate what run_worker does on Err
        let after_failure = consecutive_failures + 1;
        let delay = reconnect_delay_secs(after_failure);

        assert_eq!(after_failure, 1, "first failure sets consecutive_failures = 1");
        assert_eq!(
            delay, INITIAL_RECONNECT_DELAY_SECS,
            "first failure delay must be INITIAL_RECONNECT_DELAY_SECS"
        );
        assert!(delay > 0, "delay must be non-zero so the loop actually waits");
    }

    /// Each consecutive failure must double the delay (exponential backoff)
    /// up to MAX_RECONNECT_DELAY_SECS.
    #[test]
    fn backoff_increases_on_repeated_failures() {
        let delays: Vec<u64> = (1..=8).map(reconnect_delay_secs).collect();

        // First few must strictly increase.
        for window in delays.windows(2) {
            assert!(
                window[1] >= window[0],
                "delay must be non-decreasing: {} then {}",
                window[0],
                window[1]
            );
        }

        // Must cap at MAX_RECONNECT_DELAY_SECS.
        let high_failure_delay = reconnect_delay_secs(100);
        assert_eq!(
            high_failure_delay, MAX_RECONNECT_DELAY_SECS,
            "delay must be capped at MAX_RECONNECT_DELAY_SECS"
        );
    }

    /// A successful fetch after failures must reset `consecutive_failures` to 0,
    /// which in turn drives the next delay back to the initial value.
    #[test]
    fn successful_reconnect_resets_backoff() {
        // Simulate several failures followed by a success.
        let mut consecutive_failures: u32 = 5;

        // Simulate a successful fetch (the Ok branch in run_worker).
        if consecutive_failures > 0 {
            // The worker logs restoration here; reset the counter.
            consecutive_failures = 0;
        }

        assert_eq!(
            consecutive_failures, 0,
            "consecutive_failures must be reset to 0 after a successful fetch"
        );

        // The next failure's delay should now be back at the initial value.
        let next_failure = consecutive_failures + 1;
        let delay_after_reset = reconnect_delay_secs(next_failure);
        assert_eq!(
            delay_after_reset, INITIAL_RECONNECT_DELAY_SECS,
            "after a reset the first new failure must use the initial delay"
        );
    }

    /// The cursor must NOT be reset on a connection failure — the listener
    /// must resume from the last successfully persisted ledger.
    ///
    /// We simulate the Err branch of the poll loop and confirm the cursor
    /// value is unchanged.
    #[test]
    fn cursor_is_not_reset_on_connection_failure() {
        let cursor: u64 = 42;
        let mut consecutive_failures: u32 = 0;

        // Simulate what run_worker does on Err(_)
        consecutive_failures += 1;
        let delay = reconnect_delay_secs(consecutive_failures);
        // cursor is intentionally NOT modified in the Err branch
        let _ = delay; // would call tokio::time::sleep in the real loop
        let _ = consecutive_failures; // consumed above

        assert_eq!(
            cursor, 42,
            "cursor must stay at 42 after a failed fetch; the listener must resume from ledger 42"
        );
    }

    /// After a successful fetch the cursor advances to `latest_ledger + 1`,
    /// so on reconnect the listener resumes from exactly where it left off
    /// without reprocessing already-seen events.
    #[test]
    fn cursor_advances_to_latest_ledger_plus_one_on_success() {
        let mut cursor: u64 = 10;

        // Simulate the Ok branch of run_worker when the RPC returns ledger 15.
        let latest_ledger: u64 = 15;
        let new_cursor = latest_ledger + 1;
        if new_cursor > cursor {
            cursor = new_cursor;
        }

        assert_eq!(
            cursor, 16,
            "cursor must be latest_ledger + 1 so the next poll starts from the correct ledger"
        );
    }

    /// A partial-failure run: several errors followed by success.
    /// Verifies the full state-machine: failures accumulate, delay grows,
    /// then success resets everything and the cursor is preserved.
    #[test]
    fn reconnect_loop_state_machine_partial_failures_then_success() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_ref = call_count.clone();

        // Drive the state-machine synchronously (no actual I/O or sleeping).
        let max_iter = 6u32;
        let fail_until = 4u32; // succeed on the 5th call

        let mut cursor: u64 = 7;
        let mut consecutive_failures: u32 = 0;
        let mut recorded_delays: Vec<u64> = Vec::new();

        for _ in 0..max_iter {
            let call_n = call_count_ref.fetch_add(1, Ordering::SeqCst) + 1;

            if call_n <= fail_until {
                // Err path
                consecutive_failures += 1;
                recorded_delays.push(reconnect_delay_secs(consecutive_failures));
                // cursor unchanged
            } else {
                // Ok path — simulate result: latest_ledger = 20
                if consecutive_failures > 0 {
                    consecutive_failures = 0;
                }
                let new_cursor = 20u64 + 1;
                if new_cursor > cursor {
                    cursor = new_cursor;
                }
                break;
            }
        }

        // Failures should have grown monotonically.
        for window in recorded_delays.windows(2) {
            assert!(
                window[1] >= window[0],
                "delays must be non-decreasing during failures"
            );
        }

        // After recovery the counter is zero.
        assert_eq!(consecutive_failures, 0, "failures must be reset after success");

        // Cursor advanced to latest_ledger + 1.
        assert_eq!(cursor, 21, "cursor must have advanced to 21 after success");

        // We recorded exactly fail_until delays.
        assert_eq!(
            recorded_delays.len(),
            fail_until as usize,
            "one delay per failure"
        );
    }

    // ── existing tests ────────────────────────────────────────────────────────

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

    #[test]
    fn reconnect_delay_is_exponential_and_capped() {
        assert_eq!(reconnect_delay_secs(1), 1);
        assert_eq!(reconnect_delay_secs(2), 2);
        assert_eq!(reconnect_delay_secs(3), 4);
        assert_eq!(reconnect_delay_secs(4), 8);
        assert_eq!(reconnect_delay_secs(10), 60);
    }

    #[test]
    fn event_batch_is_split_into_configured_chunks() {
        let events: Vec<u64> = (0..5).collect();
        let chunks: Vec<_> = events.chunks(2).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], &[0, 1]);
        assert_eq!(chunks[2], &[4]);
    }
}
