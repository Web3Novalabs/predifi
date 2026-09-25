//! Stellar event listener worker.
//!
//! Polls the Stellar RPC `getEvents` endpoint every ~5 seconds (one ledger),
//! persists the latest processed ledger to the database so the worker can
//! resume after a restart, and logs every batch of events found.
//!
//! [`queue`] adds dead-letter queues, exponential-backoff retries, idempotent
//! job processing, and worker health snapshots.

pub mod queue;
pub mod stellar_listener;
/// Full contract-DB state synchronisation task (#562).
pub mod sync;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::queue::{JobQueue, RetryPolicy, DEFAULT_BASE_DELAY_MS, DEFAULT_MAX_ATTEMPTS, DEFAULT_MAX_DELAY_MS};

    #[test]
    fn retry_policy_defaults_match_expected_worker_values() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(policy.base_delay_ms, DEFAULT_BASE_DELAY_MS);
        assert_eq!(policy.max_delay_ms, DEFAULT_MAX_DELAY_MS);
        assert_eq!(policy.delay_before_attempt(1), std::time::Duration::from_millis(0));
        assert_eq!(policy.delay_before_attempt(2), std::time::Duration::from_millis(DEFAULT_BASE_DELAY_MS));
    }

    #[test]
    fn job_queue_with_defaults_uses_default_retry_policy() {
        let queue = JobQueue::with_defaults();
        let policy = queue.policy();

        assert_eq!(policy.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(policy.base_delay_ms, DEFAULT_BASE_DELAY_MS);
        assert_eq!(policy.max_delay_ms, DEFAULT_MAX_DELAY_MS);
    }

    #[test]
    fn job_kind_from_topics_maps_known_stellar_events_and_unknown_defaults() {
        let known = Some(&vec!["pool_created".to_string()]);
        assert_eq!(queue::job_kind_from_topics(known), "pool_created");

        let unknown = Some(&vec!["some_other_event".to_string()]);
        assert_eq!(queue::job_kind_from_topics(unknown), "unknown");

        let none: Option<&Vec<String>> = None;
        assert_eq!(queue::job_kind_from_topics(none), "unknown");
    }
}
