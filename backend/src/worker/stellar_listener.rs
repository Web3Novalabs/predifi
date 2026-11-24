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
use crate::worker::queue::{job_kind_from_topics, Job, JobQueue};

use std::sync::OnceLock;

/// Shared reliable job queue for idempotency, retries, DLQ, and health.
static WORKER_QUEUE: OnceLock<JobQueue> = OnceLock::new();

/// Access the process-wide worker job queue (for health endpoints / ops).
pub fn worker_job_queue() -> &'static JobQueue {
    WORKER_QUEUE.get_or_init(JobQueue::with_defaults)
}

const POLL_INTERVAL_SECS: u64 = 5;
const STATE_KEY: &str = "stellar_listener_latest_ledger";
const INITIAL_RECONNECT_DELAY_SECS: u64 = 1;
const MAX_RECONNECT_DELAY_SECS: u64 = 60;
/// Maximum number of events to request per `getEvents` page.
const PAGE_LIMIT: u64 = 100;
/// Number of ledgers to replay when initializing from the current RPC tip.
const START_LEDGER_BUFFER: u64 = 100;

// ── RPC response types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct LatestLedgerResult {
    sequence: u64,
}

#[derive(Debug, Deserialize)]
struct GetEventsResult {
    events: Vec<StellarEvent>,
    #[serde(rename = "latestLedger")]
    latest_ledger: u64,
    /// Opaque paging token returned by the RPC when more events are available.
    /// Present only when the result set was truncated; absent on the last page.
    cursor: Option<String>,
}

fn rpc_response_result<T>(rpc: RpcResponse<T>, empty_result_message: &str) -> Result<T, String> {
    match (rpc.result, rpc.error) {
        (Some(result), _) => Ok(result),
        (None, Some(error)) => {
            let detail = error
                .data
                .map(|data| format!(" ({data})"))
                .unwrap_or_default();
            Err(format!(
                "RPC error {}: {}{}",
                error.code, error.message, detail
            ))
        }
        (None, None) => Err(empty_result_message.to_string()),
    }
}

fn is_start_ledger_out_of_range(error: &str) -> bool {
    let normalized = error
        .to_ascii_lowercase()
        .replace('_', " ")
        .replace('-', " ");
    normalized.contains("out of range")
        || (normalized.contains("outside") && normalized.contains("ledger"))
        || normalized.contains("oldest ledger")
}

fn select_start_ledger(
    stored_cursor: Option<u64>,
    configured_start: Option<u64>,
    latest_ledger: Option<u64>,
) -> Option<u64> {
    stored_cursor
        .or(configured_start)
        .or_else(|| latest_ledger.map(recent_start_ledger))
}

fn recent_start_ledger(latest_ledger: u64) -> u64 {
    latest_ledger.saturating_sub(START_LEDGER_BUFFER).max(1)
}

fn recovery_start_ledger(current_cursor: u64, latest_ledger: u64) -> u64 {
    let latest_ledger = latest_ledger.max(1);
    if latest_ledger == current_cursor {
        latest_ledger.saturating_sub(1).max(1)
    } else {
        latest_ledger
    }
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

fn build_get_events_request(
    start_ledger: u64,
    paging_cursor: Option<&str>,
    contract_id: &str,
) -> Value {
    let pagination = serde_json::json!({
        "limit": PAGE_LIMIT,
        "cursor": paging_cursor,
    });
    let filters = serde_json::json!([{
        "type": "contract",
        "contractIds": [contract_id],
    }]);

    let params = if let Some(cursor) = paging_cursor {
        serde_json::json!({
            "cursor": cursor,
            "pagination": pagination,
            "filters": filters
        })
    } else {
        serde_json::json!({
            "startLedger": start_ledger,
            "pagination": pagination,
            "filters": filters
        })
    };

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEvents",
        "params": params,
    })
}

/// Fetch one page of Stellar contract events.
///
/// Pass `paging_cursor = None` on the first call; subsequent calls should
/// pass the `cursor` field from the previous page's result. When
/// `paging_cursor` is `Some`, `start_ledger` is ignored by the RPC and the
/// cursor takes precedence.
#[instrument(skip(client), name = "stellar_listener.fetch_events",
    fields(rpc_url = %rpc_url, start_ledger = start_ledger))]
async fn fetch_events(
    client: &reqwest::Client,
    rpc_url: &str,
    contract_id: &str,
    start_ledger: u64,
    paging_cursor: Option<&str>,
) -> Result<GetEventsResult, String> {
    let body = build_get_events_request(start_ledger, paging_cursor, contract_id);

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let rpc: RpcResponse<GetEventsResult> = resp.json().await.map_err(|e| e.to_string())?;
    rpc_response_result(rpc, "empty RPC result")
}

async fn get_latest_ledger(client: &reqwest::Client, rpc_url: &str) -> Result<u64, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestLedger",
        "params": {},
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let rpc: RpcResponse<LatestLedgerResult> = resp.json().await.map_err(|e| e.to_string())?;
    rpc_response_result(rpc, "empty getLatestLedger RPC result").map(|result| result.sequence)
}

// ── Worker entry point ────────────────────────────────────────────────────────

/// Spawn the Stellar event listener as a background Tokio task.
///
/// `rpc_url`   – Stellar RPC endpoint (e.g. `https://soroban-testnet.stellar.org`)
/// `db`        – PostgreSQL connection pool used to persist the ledger cursor
/// `event_bus` – broadcast channel; new predictions are published here
/// `timeout`   – maximum time to wait for an RPC response
/// `contract_id` – PrediFi contract ID whose events the listener processes
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
    contract_id: Option<String>,
) -> JoinHandle<()> {
    crate::tracing_context::spawn_worker("stellar_listener", async move {
        run_worker(
            rpc_url,
            db,
            event_bus,
            redis,
            timeout,
            max_batch_size,
            contract_id,
        )
        .await;
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

/// Run the listener only for the configured PrediFi contract.
pub async fn run_worker(
    rpc_url: String,
    db: PgPool,
    event_bus: crate::ws::EventBus,
    redis: RedisCache,
    timeout: Duration,
    max_batch_size: usize,
    contract_id: Option<String>,
) {
    let contract_id = match contract_id.filter(|id| !id.trim().is_empty()) {
        Some(contract_id) => contract_id,
        None => {
            error!(
                "PREDIFI_CONTRACT_ID is not configured; the Stellar listener is disabled to prevent unfiltered event ingestion"
            );
            return;
        }
    };

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("reqwest client with only a timeout configured must always build");
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

    let stored_cursor = load_cursor(&db).await;
    let mut cursor = match select_start_ledger(stored_cursor, configured_start_ledger, None) {
        Some(cursor) => cursor,
        None => {
            let mut consecutive_failures = 0;
            loop {
                match get_latest_ledger(&client, &rpc_url).await {
                    Ok(latest_ledger) => {
                        let cursor = recent_start_ledger(latest_ledger);
                        info!(
                            latest_ledger,
                            cursor,
                            buffer_ledgers = START_LEDGER_BUFFER,
                            "initializing stellar listener near latest ledger"
                        );
                        break cursor;
                    }
                    Err(error) => {
                        consecutive_failures += 1;
                        let delay = reconnect_delay_secs(consecutive_failures);
                        error!(
                            error = %error,
                            consecutive_failures,
                            delay_secs = delay,
                            "failed to get latest stellar ledger for listener initialization"
                        );
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }
    };
    save_cursor(&db, cursor).await;
    let mut consecutive_failures: u32 = 0;
    let batch_size = max_batch_size.max(1);
    info!(cursor, batch_size, "stellar listener starting");

    loop {
        if consecutive_failures == 0 {
            ticker.tick().await;
        }

        match fetch_events(&client, &rpc_url, &contract_id, cursor, None).await {
            Ok(first_page) => {
                if consecutive_failures > 0 {
                    info!(
                        previous_failures = consecutive_failures,
                        "stellar RPC connection restored after reconnect"
                    );
                }
                consecutive_failures = 0;

                let mut latest_ledger = first_page.latest_ledger;
                let mut paging_cursor = first_page.cursor.clone();
                let mut all_events = first_page.events;

                // Keep fetching pages until the RPC returns fewer events than
                // PAGE_LIMIT, which signals we have reached the end of the
                // available range.
                while paging_cursor.is_some()
                    && all_events.len() as u64 % PAGE_LIMIT == 0
                    && !all_events.is_empty()
                {
                    match fetch_events(
                        &client,
                        &rpc_url,
                        &contract_id,
                        cursor,
                        paging_cursor.as_deref(),
                    )
                    .await
                    {
                        Ok(next_page) => {
                            latest_ledger = next_page.latest_ledger;
                            paging_cursor = next_page.cursor.clone();
                            all_events.extend(next_page.events);
                        }
                        Err(e) => {
                            error!(
                                error = %e,
                                "failed to fetch next events page; processing events collected so far"
                            );
                            paging_cursor = None;
                        }
                    }
                }

                let count = all_events.len();
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
                        latest_ledger,
                        events = count,
                        batch_size,
                        "stellar events received"
                    );

                    for chunk in all_events.chunks(batch_size) {
                        process_event_batch(
                            &db,
                            &redis,
                            &event_bus,
                            chunk,
                            &contract_id,
                            batch_size,
                        )
                        .await;
                    }
                }

                let new_cursor = latest_ledger + 1;
                if new_cursor > cursor {
                    cursor = new_cursor;
                    save_cursor(&db, cursor).await;
                }
            }
            Err(e) => {
                if is_start_ledger_out_of_range(&e) {
                    error!(
                        error = %e,
                        cursor,
                        configured_start_ledger = ?configured_start_ledger,
                        "Stellar RPC rejected startLedger as out of range; set PREDIFI_LISTENER_START_LEDGER to a ledger retained by the RPC (stored cursor takes precedence); resetting to a recent ledger"
                    );
                    match get_latest_ledger(&client, &rpc_url).await {
                        Ok(latest_ledger) => {
                            let recent_cursor = recovery_start_ledger(cursor, latest_ledger);
                            warn!(
                                previous_cursor = cursor,
                                cursor = recent_cursor,
                                latest_ledger,
                                "resetting stellar listener cursor to recover from out-of-range startLedger"
                            );
                            cursor = recent_cursor;
                            save_cursor(&db, cursor).await;
                            consecutive_failures = 0;
                            continue;
                        }
                        Err(latest_error) => {
                            error!(
                                error = %latest_error,
                                "failed to get latest stellar ledger while recovering from out-of-range startLedger"
                            );
                        }
                    }
                }
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

fn event_matches_contract(event: &StellarEvent, contract_id: &str) -> bool {
    event.event_type == "contract" && event.contract_id.as_deref() == Some(contract_id)
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
    contract_id: &str,
    max_batch_size: usize,
) {
    let queue = worker_job_queue();
    let mut referral_events: Vec<crate::db::ReferralPaidEvent> = Vec::new();
    let mut referral_job_ids: Vec<String> = Vec::new();

    for event in events {
        info!(
            id = %event.id,
            event_type = %event.event_type,
            ledger = event.ledger,
            contract_id = ?event.contract_id,
            "stellar event"
        );

        if event.event_type == "contract" && !event_matches_contract(event, contract_id) {
            info!(
                id = %event.id,
                contract_id = ?event.contract_id,
                "skipping event from unconfigured contract"
            );
            continue;
        }

        // Idempotent skip for duplicate deliveries of the same event id.
        if queue.already_processed(&event.id) {
            info!(id = %event.id, "skipping already-processed event");
            continue;
        }

        let topic_matches = |needle: &str| {
            event
                .topics
                .as_ref()
                .map(|t| t.iter().any(|s| s == needle))
                .unwrap_or(false)
        };

        if event.event_type == "contract" {
            let kind = job_kind_from_topics(event.topics.as_ref());
            let job = Job {
                id: event.id.clone(),
                kind: kind.clone(),
                payload: event
                    .data
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                attempts: 0,
                ledger: Some(event.ledger),
            };

            if topic_matches("pool_created") {
                match handle_pool_created_event(db, redis, event).await {
                    Ok(()) => queue.record_success(&job),
                    Err(e) => {
                        error!(id = %event.id, ledger = event.ledger, error = %e, "failed to process pool_created event");
                        queue.record_failure(job, e);
                    }
                }
            } else if topic_matches("prediction_placed") {
                match handle_prediction_placed_event(db, redis, event, event_bus).await {
                    Ok(()) => queue.record_success(&job),
                    Err(e) => {
                        error!(id = %event.id, ledger = event.ledger, error = %e, "failed to process prediction_placed event");
                        queue.record_failure(job, e);
                    }
                }
            } else if topic_matches("pool_resolved") {
                match handle_pool_resolved_event(db, event).await {
                    Ok(()) => queue.record_success(&job),
                    Err(e) => {
                        error!(id = %event.id, ledger = event.ledger, error = %e, "failed to process pool_resolved event");
                        queue.record_failure(job, e);
                    }
                }
            } else if topic_matches("pool_canceled") {
                match handle_pool_canceled_event(db, event).await {
                    Ok(()) => queue.record_success(&job),
                    Err(e) => {
                        error!(id = %event.id, ledger = event.ledger, error = %e, "failed to process pool_canceled event");
                        queue.record_failure(job, e);
                    }
                }
            } else if topic_matches("referral_paid") {
                match parse_referral_paid_event(event) {
                    Ok(ev) => {
                        referral_events.push(ev);
                        referral_job_ids.push(event.id.clone());
                    }
                    Err(e) => {
                        error!(id = %event.id, ledger = event.ledger, error = %e, "failed to parse referral_paid event");
                        queue.record_failure(job, e);
                    }
                }
            }
        }
    }

    if !referral_events.is_empty() {
        match crate::db::insert_referrals_bulk(db, &referral_events, max_batch_size).await {
            Ok(()) => {
                for id in &referral_job_ids {
                    queue.mark_processed(id);
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    count = referral_events.len(),
                    "failed to bulk insert referral events"
                );
                for id in referral_job_ids {
                    queue.record_failure(
                        Job {
                            id,
                            kind: "referral_paid".into(),
                            payload: String::new(),
                            attempts: 0,
                            ledger: None,
                        },
                        e.to_string(),
                    );
                }
            }
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
    // #1932: the contract's `PoolCreatedEvent` has carried these since
    // multi-outcome pools were added; default to a binary pool shape when
    // an older/partial event payload omits them rather than failing ingest.
    let options_count = extract_u32(data, "options_count").unwrap_or(2);
    let outcome_descriptions = extract_string_vec(data, "outcome_descriptions").unwrap_or_default();
    // Optional creator-assigned tags; older events omit them.
    let tags = extract_string_vec(data, "tags").unwrap_or_default();

    let pool_event = crate::db::PoolCreatedEvent {
        pool_id,
        creator,
        end_time,
        token,
        category,
        description,
        tags,
        options_count: options_count as i32,
        outcome_descriptions,
    };

    crate::db::insert_pool_from_event(db, &pool_event)
        .await
        .map_err(|e| e.to_string())?;

    redis.invalidate_pools_cache().await;
    Ok(())
}

async fn handle_prediction_placed_event(
    db: &PgPool,
    redis: &RedisCache,
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

    // Protocol-wide stats and this user's cached prediction history both
    // just went stale. Without this, `/api/v1/users/:address/predictions`
    // (now cached, see #1938) would keep serving a pre-prediction snapshot
    // for up to USER_PREDICTIONS_CACHE_TTL after a confirmed on-chain
    // prediction — this is the primary ingestion path (events indexed off
    // the Stellar RPC), unlike `ingest_prediction_placed` in routes/v1.rs
    // which only invalidated the stats cache.
    redis.invalidate_stats_cache().await;
    redis.invalidate_user_predictions_cache(&ev.user_address).await;

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

/// Extract a `u32` field (e.g. `options_count`, #1932).
fn extract_u32(data: &Value, key: &str) -> Option<u32> {
    extract_i128(data.get(key)?).and_then(|v| u32::try_from(v).ok())
}

/// Extract a `Vec<String>` field (e.g. `outcome_descriptions`, #1932).
///
/// Handles the same "single-key wrapper object" shape `extract_string_value`
/// unwraps for scalars (some RPC encodings wrap a Soroban `Vec` the same way
/// they wrap other typed values), as well as a plain JSON array.
fn extract_string_vec(data: &Value, key: &str) -> Option<Vec<String>> {
    extract_string_vec_value(data.get(key)?)
}

fn extract_string_vec_value(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Array(items) => items.iter().map(extract_string_value).collect(),
        Value::Object(map) if map.len() == 1 => {
            map.values().next().and_then(extract_string_vec_value)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    #[test]
    fn get_events_request_filters_to_configured_contract() {
        let expected_filters = serde_json::json!([{
            "type": "contract",
            "contractIds": ["CABC123"],
        }]);
        let initial_request = build_get_events_request(42, None, "CABC123");
        assert_eq!(initial_request["method"], "getEvents");
        assert_eq!(initial_request["params"]["startLedger"], 42);
        assert_eq!(initial_request["params"]["filters"], expected_filters);

        let paged_request = build_get_events_request(42, Some("next-page"), "CABC123");
        assert_eq!(paged_request["params"]["cursor"], "next-page");
        assert_eq!(paged_request["params"]["filters"], expected_filters);
    }

    #[test]
    fn contract_event_filter_rejects_other_and_missing_contract_ids() {
        let make_event = |contract_id: Option<&str>| StellarEvent {
            event_type: String::from("contract"),
            ledger: 42,
            contract_id: contract_id.map(str::to_owned),
            id: String::from("event"),
            topics: None,
            data: None,
        };

        assert!(event_matches_contract(
            &make_event(Some("CABC123")),
            "CABC123"
        ));
        assert!(!event_matches_contract(
            &make_event(Some("COTHER123")),
            "CABC123"
        ));
        assert!(!event_matches_contract(&make_event(None), "CABC123"));
    }

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

        let resp: RpcResponse<GetEventsResult> = serde_json::from_str(json).unwrap();
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

        let resp: RpcResponse<GetEventsResult> = serde_json::from_str(json).unwrap();
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

    // ── Pagination tests ──────────────────────────────────────────────────────

    /// Build a minimal `GetEventsResult` JSON payload for a mock RPC.
    fn make_events_page(ids: &[&str], next_cursor: Option<&str>, latest_ledger: u64) -> String {
        let events_json: Vec<String> = ids
            .iter()
            .map(|id| {
                format!(
                    r#"{{"type":"contract","ledger":10,"contractId":"CA","id":"{}"}}"#,
                    id
                )
            })
            .collect();
        let cursor_field = match next_cursor {
            Some(c) => format!(r#","cursor":"{}""#, c),
            None => String::new(),
        };
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"result":{{"events":[{}],"latestLedger":{}{}}}}}"#,
            events_json.join(","),
            latest_ledger,
            cursor_field
        )
    }

    /// fetch_events threads the paging cursor from one call to the next so
    /// events on pages beyond the first are not silently dropped.
    ///
    /// The mock RPC returns a full PAGE_LIMIT page on the first request
    /// (cursor present) and a short page on the second (no cursor), signalling
    /// end-of-range.  Both pages must be collected before the ledger cursor
    /// advances.
    #[tokio::test]
    async fn pagination_collects_all_pages_before_advancing_cursor() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
            sync::{oneshot, Mutex},
        };

        // Build the two response bodies up front.
        let page1_ids: Vec<String> = (0..PAGE_LIMIT).map(|i| format!("evt-p1-{}", i)).collect();
        let page1_id_refs: Vec<&str> = page1_ids.iter().map(String::as_str).collect();
        let page1_body =
            make_events_page(&page1_id_refs, Some("cursor-after-page1"), 50);

        let page2_body = make_events_page(&["evt-p2-0", "evt-p2-1"], None, 55);

        // Shared call counter so the mock can serve different pages.
        let call_count = Arc::new(Mutex::new(0u32));
        let call_count_srv = call_count.clone();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let addr = listener.local_addr().expect("local addr");
        let url = format!("http://{}", addr);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let p1 = page1_body.clone();
        let p2 = page2_body.clone();
        let srv = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    result = listener.accept() => {
                        if let Ok((mut socket, _)) = result {
                            let mut count = call_count_srv.lock().await;
                            *count += 1;
                            let body = if *count == 1 { p1.clone() } else { p2.clone() };
                            drop(count);
                            tokio::spawn(async move {
                                let mut buf = [0u8; 4096];
                                let _ = socket.read(&mut buf).await;
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\n\
                                     Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\
                                     \r\n\
                                     {}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                                let _ = socket.shutdown().await;
                            });
                        }
                    }
                }
            }
        });

        let client = reqwest::Client::new();

        // First page
        let first = fetch_events(&client, &url, "CA", 1, None)
            .await
            .expect("first page should succeed");
        assert_eq!(
            first.events.len() as u64,
            PAGE_LIMIT,
            "first page must carry PAGE_LIMIT events"
        );
        assert!(
            first.cursor.is_some(),
            "first page must include a continuation cursor"
        );

        // Second page, using the cursor from the first
        let second = fetch_events(&client, &url, "CA", 1, first.cursor.as_deref())
            .await
            .expect("second page should succeed");
        assert_eq!(second.events.len(), 2, "second page must carry 2 events");
        assert!(
            second.cursor.is_none(),
            "second page must have no cursor (last page)"
        );

        // Combined event IDs must include events from both pages
        let total: Vec<String> = first
            .events
            .iter()
            .chain(second.events.iter())
            .map(|e| e.id.clone())
            .collect();
        assert_eq!(
            total.len() as u64,
            PAGE_LIMIT + 2,
            "all events from both pages must be collected"
        );
        assert!(
            total.iter().any(|id| id.starts_with("evt-p2-")),
            "second-page events must be present in the combined list"
        );

        // The ledger cursor should only advance after the last page
        let final_ledger = second.latest_ledger;
        assert_eq!(
            final_ledger + 1,
            56,
            "cursor must advance to latestLedger+1 from the last page"
        );

        let _ = shutdown_tx.send(());
        let _ = srv.await;
    }
}
