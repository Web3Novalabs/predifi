# PrediFi Documentation Index

This index lists every document in `docs/` with a one-line description,
grouped by topic.

## Getting Started

- [Quickstart: Make Your First Prediction in 5 Minutes](quickstart.md) — fastest path to placing a prediction.
- [Local Development Environment](local-development.md) — set up the backend, frontend, and tooling locally.
- [Initialize Backend Cargo Workspace](initialize-backend-cargo-workspace.md) — bootstrap the Rust/Cargo workspace.
- [OpenAPI Spec Validation & TypeScript Client Generation](openapi-client-generation.md) — generate and validate the API client.

## Architecture

- [PrediFi Architecture Overview](ARCHITECTURE_OVERVIEW.md) — high-level system design.
- [Prediction Lifecycle](prediction-lifecycle.md) — how a prediction moves through the system.
- [Verifiable Oracles](oracles.md) — oracle design and verification.
- [Frontend Features & Enhancements](FRONTEND_FEATURES.md) — UI capabilities and roadmap.

## Contracts

- [Contract Reference](contract-reference.md) — on-chain contract surfaces and entrypoints.
- [PrediFi Smart Contract Deployment Guide](SMART_CONTRACT_DEPLOYMENT_GUIDE.md) — deploy the contracts.

## Operations

- [Health Check Endpoint](health-check-endpoint.md) — liveness/readiness endpoint.
- [Advanced Health Checks (DB/RPC/Redis)](ADVANCED_HEALTH_CHECKS.md) — deeper dependency checks.
- [Troubleshooting](troubleshooting.md) — common failures and fixes.
- [Whitelist Events](whitelist-events.md) — whitelist event reference.
- [Error Handling Reference](ERROR_HANDLING_REFERENCE.md) — error codes and handling.
- [Accessibility (WCAG 2.1 AA)](ACCESSIBILITY.md) — accessibility conformance notes.

## Security

- [Security Audit: Access Control for Admin Functions](security-access-control-audit.md)
- [Security Analysis: Front-Running Protection for Predictions](security-front-running-analysis.md)
- [Security Analysis: Price Feed Manipulation Resistance](security-price-feed-analysis.md)
- [Security Analysis: Reentrancy](security-reentrancy-analysis.md)
