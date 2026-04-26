//! Stellar event listener worker.
//!
//! Polls the Stellar RPC `getEvents` endpoint every ~5 seconds (one ledger),
//! persists the latest processed ledger to the database so the worker can
//! resume after a restart, and logs every batch of events found.

pub mod stellar_listener;
