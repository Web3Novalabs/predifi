//! Reliable background job processing for the Stellar event worker.
//!
//! Provides:
//! - Exponential backoff retry policies
//! - Dead-letter queue (DLQ) for permanently failed jobs
//! - Idempotent processing via a processed-job ID set
//! - Worker health snapshots (last success, consecutive failures, DLQ depth)

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// Default maximum delivery attempts before a job is dead-lettered.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;
/// Base delay (ms) for the first retry.
pub const DEFAULT_BASE_DELAY_MS: u64 = 200;
/// Cap on exponential backoff delay (ms).
pub const DEFAULT_MAX_DELAY_MS: u64 = 30_000;
/// Maximum retained DLQ entries (oldest dropped when exceeded).
const DLQ_CAPACITY: usize = 1_000;
/// Maximum retained processed job IDs for idempotency.
const IDEMPOTENCY_CAPACITY: usize = 10_000;

/// Retry policy with exponential backoff.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_DELAY_MS,
        }
    }
}

impl RetryPolicy {
    /// Delay before attempt `n` (1-indexed). Attempt 1 has zero delay.
    pub fn delay_before_attempt(&self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return Duration::from_millis(0);
        }
        let shift = attempt.saturating_sub(2).min(16);
        let ms = self
            .base_delay_ms
            .saturating_mul(2u64.saturating_pow(shift))
            .min(self.max_delay_ms);
        Duration::from_millis(ms)
    }

    /// Whether another attempt is allowed after `attempt` failures.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// A unit of work processed by the worker queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Stable unique id used for idempotent delivery (e.g. Stellar event id).
    pub id: String,
    /// Logical job kind (e.g. `pool_created`, `prediction_placed`).
    pub kind: String,
    /// Opaque payload (typically JSON).
    pub payload: String,
    /// Delivery attempts so far (starts at 0 before first run).
    pub attempts: u32,
    /// Optional ledger for diagnostics.
    pub ledger: Option<u64>,
}

/// Entry parked in the dead-letter queue after exhausting retries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub job: Job,
    pub last_error: String,
    pub failed_at_unix_ms: u64,
}

/// Point-in-time health view for monitoring /health integrations.
#[derive(Debug, Clone, Serialize)]
pub struct WorkerHealth {
    pub last_success_unix_ms: Option<u64>,
    pub last_failure_unix_ms: Option<u64>,
    pub consecutive_failures: u64,
    pub jobs_processed: u64,
    pub jobs_dead_lettered: u64,
    pub dlq_depth: usize,
    pub pending_retries: usize,
    pub is_healthy: bool,
}

/// Shared reliable queue used by the Stellar listener.
#[derive(Clone)]
pub struct JobQueue {
    inner: Arc<QueueInner>,
}

struct QueueInner {
    policy: RetryPolicy,
    /// Jobs waiting to be retried (id → next eligible Instant).
    retry_after: Mutex<HashMap<String, Instant>>,
    pending: Mutex<VecDeque<Job>>,
    dlq: Mutex<VecDeque<DeadLetterEntry>>,
    /// Recently processed job ids for duplicate-delivery safety.
    processed: Mutex<HashSet<String>>,
    processed_order: Mutex<VecDeque<String>>,
    last_success_ms: AtomicU64,
    last_failure_ms: AtomicU64,
    consecutive_failures: AtomicU64,
    jobs_processed: AtomicU64,
    jobs_dead_lettered: AtomicU64,
}

impl JobQueue {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            inner: Arc::new(QueueInner {
                policy,
                retry_after: Mutex::new(HashMap::new()),
                pending: Mutex::new(VecDeque::new()),
                dlq: Mutex::new(VecDeque::new()),
                processed: Mutex::new(HashSet::new()),
                processed_order: Mutex::new(VecDeque::new()),
                last_success_ms: AtomicU64::new(0),
                last_failure_ms: AtomicU64::new(0),
                consecutive_failures: AtomicU64::new(0),
                jobs_processed: AtomicU64::new(0),
                jobs_dead_lettered: AtomicU64::new(0),
            }),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RetryPolicy::default())
    }

    pub fn policy(&self) -> RetryPolicy {
        self.inner.policy
    }

    /// Returns `true` if this job id was already successfully processed.
    pub fn already_processed(&self, job_id: &str) -> bool {
        self.inner
            .processed
            .lock()
            .expect("processed lock")
            .contains(job_id)
    }

    /// Mark a job id as successfully processed (idempotency bookkeeping).
    pub fn mark_processed(&self, job_id: &str) {
        let mut processed = self.inner.processed.lock().expect("processed lock");
        let mut order = self.inner.processed_order.lock().expect("order lock");
        if processed.insert(job_id.to_string()) {
            order.push_back(job_id.to_string());
            while order.len() > IDEMPOTENCY_CAPACITY {
                if let Some(old) = order.pop_front() {
                    processed.remove(&old);
                }
            }
        }
    }

    /// Enqueue a job for (re)processing. No-ops if already processed.
    pub fn enqueue(&self, job: Job) -> bool {
        if self.already_processed(&job.id) {
            info!(job_id = %job.id, kind = %job.kind, "skipping duplicate job (idempotent)");
            return false;
        }
        self.inner
            .pending
            .lock()
            .expect("pending lock")
            .push_back(job);
        true
    }

    /// Pop the next ready job, respecting retry backoff windows.
    pub fn dequeue_ready(&self) -> Option<Job> {
        let now = Instant::now();
        let mut pending = self.inner.pending.lock().expect("pending lock");
        let retry_after = self.inner.retry_after.lock().expect("retry lock");

        let mut skipped = VecDeque::new();
        let mut found = None;
        while let Some(job) = pending.pop_front() {
            if let Some(ready_at) = retry_after.get(&job.id) {
                if *ready_at > now {
                    skipped.push_back(job);
                    continue;
                }
            }
            found = Some(job);
            break;
        }
        for job in skipped {
            pending.push_back(job);
        }
        found
    }

    /// Record a successful processing outcome.
    pub fn record_success(&self, job: &Job) {
        self.mark_processed(&job.id);
        self.inner
            .retry_after
            .lock()
            .expect("retry lock")
            .remove(&job.id);
        self.inner
            .last_success_ms
            .store(unix_ms_now(), Ordering::Relaxed);
        self.inner
            .consecutive_failures
            .store(0, Ordering::Relaxed);
        self.inner.jobs_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failure: schedule retry or move to DLQ when attempts are exhausted.
    pub fn record_failure(&self, mut job: Job, error: impl Into<String>) {
        let error = error.into();
        job.attempts = job.attempts.saturating_add(1);
        self.inner
            .last_failure_ms
            .store(unix_ms_now(), Ordering::Relaxed);
        self.inner
            .consecutive_failures
            .fetch_add(1, Ordering::Relaxed);

        if self.inner.policy.should_retry(job.attempts) {
            let delay = self.inner.policy.delay_before_attempt(job.attempts + 1);
            warn!(
                job_id = %job.id,
                kind = %job.kind,
                attempts = job.attempts,
                delay_ms = delay.as_millis() as u64,
                error = %error,
                "job failed; scheduling retry"
            );
            self.inner
                .retry_after
                .lock()
                .expect("retry lock")
                .insert(job.id.clone(), Instant::now() + delay);
            self.inner
                .pending
                .lock()
                .expect("pending lock")
                .push_back(job);
        } else {
            error!(
                job_id = %job.id,
                kind = %job.kind,
                attempts = job.attempts,
                error = %error,
                "job exhausted retries; moving to dead-letter queue"
            );
            self.push_dlq(DeadLetterEntry {
                job,
                last_error: error,
                failed_at_unix_ms: unix_ms_now(),
            });
        }
    }

    fn push_dlq(&self, entry: DeadLetterEntry) {
        let mut dlq = self.inner.dlq.lock().expect("dlq lock");
        if dlq.len() >= DLQ_CAPACITY {
            dlq.pop_front();
        }
        dlq.push_back(entry);
        self.inner
            .jobs_dead_lettered
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot of DLQ contents (for ops / replay tooling).
    pub fn dead_letters(&self) -> Vec<DeadLetterEntry> {
        self.inner
            .dlq
            .lock()
            .expect("dlq lock")
            .iter()
            .cloned()
            .collect()
    }

    /// Re-queue a dead-lettered job for another attempt (clears attempt count).
    pub fn requeue_from_dlq(&self, job_id: &str) -> bool {
        let mut dlq = self.inner.dlq.lock().expect("dlq lock");
        if let Some(pos) = dlq.iter().position(|e| e.job.id == job_id) {
            let mut entry = dlq.remove(pos).expect("index valid");
            entry.job.attempts = 0;
            drop(dlq);
            self.enqueue(entry.job);
            return true;
        }
        false
    }

    /// Health snapshot for monitoring endpoints.
    pub fn health(&self) -> WorkerHealth {
        let consecutive = self.inner.consecutive_failures.load(Ordering::Relaxed);
        let last_success = nonzero_ms(self.inner.last_success_ms.load(Ordering::Relaxed));
        let last_failure = nonzero_ms(self.inner.last_failure_ms.load(Ordering::Relaxed));
        let dlq_depth = self.inner.dlq.lock().expect("dlq lock").len();
        let pending_retries = self.inner.pending.lock().expect("pending lock").len();

        // Unhealthy if we have never succeeded and already failed, or many consecutive failures.
        let is_healthy = consecutive < 5
            && (last_success.is_some() || self.inner.jobs_processed.load(Ordering::Relaxed) == 0);

        WorkerHealth {
            last_success_unix_ms: last_success,
            last_failure_unix_ms: last_failure,
            consecutive_failures: consecutive,
            jobs_processed: self.inner.jobs_processed.load(Ordering::Relaxed),
            jobs_dead_lettered: self.inner.jobs_dead_lettered.load(Ordering::Relaxed),
            dlq_depth,
            pending_retries,
            is_healthy,
        }
    }

    /// Run `handler` with retry/DLQ/idempotency semantics for a single job.
    pub async fn process_with_retry<F, Fut>(&self, mut job: Job, handler: F) -> Result<(), String>
    where
        F: Fn(Job) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        if self.already_processed(&job.id) {
            info!(job_id = %job.id, "idempotent skip");
            return Ok(());
        }

        loop {
            let delay = self.inner.policy.delay_before_attempt(job.attempts + 1);
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }

            match handler(job.clone()).await {
                Ok(()) => {
                    self.record_success(&job);
                    return Ok(());
                }
                Err(e) => {
                    job.attempts = job.attempts.saturating_add(1);
                    if !self.inner.policy.should_retry(job.attempts) {
                        self.inner
                            .last_failure_ms
                            .store(unix_ms_now(), Ordering::Relaxed);
                        self.inner
                            .consecutive_failures
                            .fetch_add(1, Ordering::Relaxed);
                        self.push_dlq(DeadLetterEntry {
                            job: job.clone(),
                            last_error: e.clone(),
                            failed_at_unix_ms: unix_ms_now(),
                        });
                        return Err(e);
                    }
                    warn!(
                        job_id = %job.id,
                        attempts = job.attempts,
                        error = %e,
                        "retrying job"
                    );
                    self.inner
                        .last_failure_ms
                        .store(unix_ms_now(), Ordering::Relaxed);
                    self.inner
                        .consecutive_failures
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn nonzero_ms(v: u64) -> Option<u64> {
    if v == 0 {
        None
    } else {
        Some(v)
    }
}

/// Infer a job kind string from Stellar event topics.
pub fn job_kind_from_topics(topics: Option<&Vec<String>>) -> String {
    topics
        .and_then(|t| {
            t.iter()
                .find(|s| {
                    matches!(
                        s.as_str(),
                        "pool_created"
                            | "prediction_placed"
                            | "pool_resolved"
                            | "pool_canceled"
                            | "referral_paid"
                    )
                })
                .cloned()
        })
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_exponential_and_capped() {
        let policy = RetryPolicy {
            max_attempts: 10,
            base_delay_ms: 200,
            max_delay_ms: 5_000,
        };
        assert_eq!(policy.delay_before_attempt(1), Duration::from_millis(0));
        assert_eq!(policy.delay_before_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_before_attempt(3), Duration::from_millis(400));
        assert_eq!(policy.delay_before_attempt(4), Duration::from_millis(800));
        assert_eq!(policy.delay_before_attempt(10), Duration::from_millis(5_000));
    }

    #[test]
    fn should_retry_respects_max_attempts() {
        let policy = RetryPolicy::default();
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(4));
        assert!(!policy.should_retry(5));
    }

    #[test]
    fn idempotent_skip_on_duplicate_enqueue() {
        let q = JobQueue::with_defaults();
        let job = Job {
            id: "evt-1".into(),
            kind: "pool_created".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: Some(1),
        };
        assert!(q.enqueue(job.clone()));
        q.mark_processed("evt-1");
        assert!(!q.enqueue(job));
        assert!(q.already_processed("evt-1"));
    }

    #[test]
    fn exhausted_retries_go_to_dlq() {
        let q = JobQueue::new(RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 1,
            max_delay_ms: 1,
        });
        let job = Job {
            id: "evt-fail".into(),
            kind: "prediction_placed".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: None,
        };
        q.record_failure(job.clone(), "boom");
        assert_eq!(q.dead_letters().len(), 0); // first failure → retry
        let mut retried = q.dequeue_ready().expect("pending retry");
        retried.attempts = 1;
        q.record_failure(retried, "boom again");
        assert_eq!(q.dead_letters().len(), 1);
        assert_eq!(q.health().jobs_dead_lettered, 1);
    }

    #[test]
    fn requeue_from_dlq_resets_attempts() {
        let q = JobQueue::new(RetryPolicy {
            max_attempts: 1,
            base_delay_ms: 1,
            max_delay_ms: 1,
        });
        let job = Job {
            id: "evt-dlq".into(),
            kind: "pool_resolved".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: None,
        };
        q.record_failure(job, "fatal");
        assert_eq!(q.dead_letters().len(), 1);
        assert!(q.requeue_from_dlq("evt-dlq"));
        let again = q.dequeue_ready().expect("requeued");
        assert_eq!(again.attempts, 0);
        assert!(q.dead_letters().is_empty());
    }

    #[test]
    fn health_reports_success() {
        let q = JobQueue::with_defaults();
        let job = Job {
            id: "ok".into(),
            kind: "pool_created".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: None,
        };
        q.record_success(&job);
        let h = q.health();
        assert!(h.is_healthy);
        assert_eq!(h.jobs_processed, 1);
        assert_eq!(h.consecutive_failures, 0);
        assert!(h.last_success_unix_ms.is_some());
    }

    #[tokio::test]
    async fn process_with_retry_succeeds_after_transient_errors() {
        let q = JobQueue::new(RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 1,
            max_delay_ms: 1,
        });
        let attempts = Arc::new(AtomicU64::new(0));
        let attempts_c = attempts.clone();
        let job = Job {
            id: "retry-ok".into(),
            kind: "test".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: None,
        };
        let result = q
            .process_with_retry(job, |_j| {
                let c = attempts_c.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                    if n < 3 {
                        Err("transient".into())
                    } else {
                        Ok(())
                    }
                }
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert!(q.already_processed("retry-ok"));
    }

    #[tokio::test]
    async fn process_with_retry_dead_letters_on_permanent_failure() {
        let q = JobQueue::new(RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 1,
            max_delay_ms: 1,
        });
        let job = Job {
            id: "perm-fail".into(),
            kind: "test".into(),
            payload: "{}".into(),
            attempts: 0,
            ledger: None,
        };
        let result = q
            .process_with_retry(job, |_j| async { Err("always".into()) })
            .await;
        assert!(result.is_err());
        assert_eq!(q.dead_letters().len(), 1);
        assert!(!q.already_processed("perm-fail"));
    }
}
