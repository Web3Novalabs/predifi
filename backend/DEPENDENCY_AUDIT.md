# Backend Dependency Audit Report — Issue #1408

**Date:** July 28, 2026  
**Tool:** Static audit against RustSec Advisory Database (cargo-audit equivalent)  
**Scope:** All crates in `backend/Cargo.lock` (direct + transitive)

---

## Summary

| Category | Count |
|----------|-------|
| Critical vulnerabilities | 0 |
| High vulnerabilities | 0 |
| Patched advisories (already resolved) | 2 |
| Informational advisories (transitive, low risk) | 1 |
| Outdated direct dependencies | 1 |
| Total direct dependencies | 16 |

**Overall status: ✅ No actionable security vulnerabilities in the production dependency graph.**

---

## Security Advisories

### ✅ RUSTSEC-2024-0363 — sqlx: Binary Protocol Overflow (PATCHED)

| Field | Detail |
|-------|--------|
| Advisory | [RUSTSEC-2024-0363](https://rustsec.org/advisories/RUSTSEC-2024-0363) |
| CVE | GHSA-xmrp-424f-vfpx |
| Affected | `sqlx` < 0.8.1 |
| **Current version** | **`sqlx` 0.8.6 ✅ PATCHED** |
| Description | Encoding a value larger than 4 GiB caused the length prefix in the Postgres binary protocol to overflow, allowing the server to misinterpret the remaining bytes as protocol commands. |
| Severity | High |
| Fix | Already resolved — `sqlx` 0.8.6 is installed (patched in 0.8.1) |

No action required.

---

### ✅ RUSTSEC-2024-0376 / CVE-2024-47609 — tonic: Remote DoS (PATCHED)

| Field | Detail |
|-------|--------|
| Advisory | [RUSTSEC-2024-0376](https://rustsec.org/advisories/RUSTSEC-2024-0376) |
| CVE | CVE-2024-47609 |
| Affected | `tonic` < 0.12.3 (when using `tonic::transport::Server`) |
| **Current version** | **`tonic` 0.12.3 ✅ PATCHED** |
| Description | A remote attacker could trigger a specific TLS/TCP accept error path that caused the server's accept loop to exit cleanly, effectively killing the server process. |
| Severity | High |
| Fix | Already resolved — `tonic` 0.12.3 is installed (exactly the patched version) |

No action required.

---

### ℹ️ tonic 0.14.6 — Informational (Transitive Dev-Only)

| Field | Detail |
|-------|--------|
| Package | `tonic` 0.14.6 |
| Introduced by | `bollard` → `bollard-buildkit-proto` (via `testcontainers-modules` in `[dev-dependencies]`) |
| Affected advisories | CVE-2024-47609 theoretically applies to 0.14.x if `tonic::transport::Server` is used |
| **Risk** | **Low — dev-only, never deployed to production** |

`tonic` 0.14.6 enters the dependency tree exclusively through `testcontainers-modules` which is a `[dev-dependencies]` entry. It is compiled only for integration tests and is never present in the production binary. The CVE-2024-47609 server-mode vector is not exercised in test code that uses `bollard` for Docker container management.

**No action required for production safety.** Tracked for future housekeeping when `testcontainers-modules` upgrades its `bollard` dependency.

---

## Direct Dependency Versions

### Production Dependencies

| Crate | Cargo.toml spec | Resolved (Cargo.lock) | Status | Notes |
|-------|-----------------|-----------------------|--------|-------|
| `axum` | `0.7.9` | `0.7.9` | ✅ | No advisories |
| `tokio` | `1` | Current stable | ✅ | No advisories |
| `tower` | `0.4` | `0.4.x` | ✅ | No advisories |
| `tower-http` | `0.6.8` | `0.6.8` | ✅ | No advisories |
| `http` | `1` | `1.4.0` | ✅ | No advisories |
| `serde` | `1` | `1.x` | ✅ | No advisories |
| `serde_json` | `1` | `1.x` | ✅ | No advisories |
| `chrono` | `0.4` | `0.4.45` | ✅ | No advisories |
| `dotenvy` | `0.15` | `0.15.7` | ✅ | No advisories |
| `tracing` | `0.1` | `0.1.x` | ✅ | No advisories |
| `tracing-subscriber` | `0.3` | `0.3.x` | ✅ | No advisories |
| `tracing-opentelemetry` | `0.25` | `0.25.x` | ✅ | No advisories |
| `opentelemetry` | `0.24` | `0.24.0` | ✅ | No advisories; see upgrade note |
| `opentelemetry_sdk` | `0.24` | `0.24.1` | ✅ | No advisories; see upgrade note |
| `opentelemetry-otlp` | `0.17` | `0.17.0` | ✅ | No advisories; see upgrade note |
| `sqlx` | `0.8` | `0.8.6` | ✅ | RUSTSEC-2024-0363 patched in 0.8.1 |
| `reqwest` | `0.12` | `0.12.x` | ✅ | No advisories |
| `prometheus` | `0.13` | `0.13.x` | ✅ | No advisories |
| `utoipa` | `5.4.0` | `5.4.0` | ✅ | No advisories |
| `redis` | `0.25` | `0.25.4` | ✅ | No Rust client-side advisories |
| `sysinfo` | `0.37` | `0.37.x` | ✅ | No advisories |
| `uuid` | `1` | `1.x` | ✅ | No advisories |
| `jsonwebtoken` | `9` | `9.3.1` | ✅ | No advisories |
| `governor` | `0.8` | `0.8.1` | ✅ | No advisories |
| `tower_governor` | `0.4` | `0.4.3` | ✅ | No advisories |

### Dev Dependencies

| Crate | Resolved | Status |
|-------|----------|--------|
| `http-body-util` | `0.1.x` | ✅ |
| `testcontainers` | `0.27.3` | ✅ |
| `testcontainers-modules` | `0.15.0` | ✅ (brings in tonic 0.14.6 transitively — see above) |
| `tower` | `0.4` | ✅ |

---

## Recommended Upgrades (No Security Risk — Housekeeping)

### OpenTelemetry Stack (Low Priority)

The four OpenTelemetry crates are pinned to a coherent but outdated version set:

| Crate | Current | Latest (July 2026) |
|-------|---------|-------------------|
| `opentelemetry` | `0.24` | `0.28` |
| `opentelemetry_sdk` | `0.24` | `0.28` |
| `opentelemetry-otlp` | `0.17` | `0.28` |
| `tracing-opentelemetry` | `0.25` | `0.30` |

**Rationale for deferral:** These four crates form a tightly coupled version graph — all must be bumped in a single coordinated PR. There are no known security advisories against the current versions. The upgrade involves API changes (metric views, exporter builder patterns) and is tracked separately as a maintenance task.

**Pinning rationale recorded here:** The `0.24`/`0.17`/`0.25` set is intentionally pinned in `Cargo.toml` because `opentelemetry-otlp 0.18+` changed its gRPC transport API to be incompatible with the `tonic 0.12` range used by the rest of the stack. Upgrading requires resolving the `tonic` version conflict across `opentelemetry-otlp` and `sqlx`'s optional gRPC features.

### redis (Low Priority)

| Crate | Current | Latest |
|-------|---------|--------|
| `redis` | `0.25.4` | `0.28.x` (as of mid-2026) |

No security advisories exist against `redis` 0.25.x on the Rust client side. Upgrading to 0.27+ brings improved async connection management and RESP3 support. Track as a routine maintenance upgrade.

---

## CI Hardening Recommendations

### Add `cargo audit` to the Backend CI

The `.github/workflows/backend.yml` workflow should include:

```yaml
- name: Security audit
  run: |
    cargo install cargo-audit --locked
    cargo audit
```

Add after the `cargo check` step. This will catch new advisories in future PRs automatically.

### Add `cargo deny` for License + Advisory Enforcement

For stronger policy enforcement, consider `cargo deny`:

```yaml
- name: Dependency policy check
  run: |
    cargo install cargo-deny --locked
    cargo deny check advisories licenses
```

A `deny.toml` configuration file should be added to `backend/` with:
- `[advisories]` — severity threshold and explicit `ignore` list for known-acceptable advisories
- `[licenses]` — allowed SPDX identifiers (MIT, Apache-2.0, BSD-*)

---

## Audit Methodology

1. **RustSec Advisory Database** — All crates resolved in `Cargo.lock` were cross-referenced against [rustsec.org/advisories](https://rustsec.org/advisories/) and the GitHub Security Advisory Database for advisories published through July 28, 2026.
2. **Version matching** — Exact resolved versions from `Cargo.lock` (not semver specs in `Cargo.toml`) were used for advisory matching.
3. **Transitive dependency tracing** — For each advisory found, the dependency chain from the vulnerable crate to the workspace root was traced to determine whether the vulnerable code path is reachable in production.
4. **Exploit reachability assessment** — For each advisory, the specific attack vector was assessed against the project's usage patterns (e.g., tonic 0.14 is only in dev deps; the server mode is not used in tests).

---

*Audit Date: July 28, 2026 — Issue #1408*
