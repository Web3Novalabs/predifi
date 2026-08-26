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
