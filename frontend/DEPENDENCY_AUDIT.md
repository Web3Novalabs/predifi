# Dependency Audit Report

This document consolidates two separate audit passes:

1. **Issue #955** (June 2026) — initial audit confirming all listed packages were in use
2. **Issue #1408** (July 2026) — security vulnerability scan, version updates, and deprecation removals

---

## Issue #1408 Audit Summary (July 28, 2026)

### ⚠️ Critical Security Vulnerabilities Found and Fixed

| CVE | Severity | Package | Vulnerable Range | Fixed Version | Description |
|-----|----------|---------|-----------------|---------------|-------------|
| CVE-2025-66478 | **Critical (CVSS 10.0)** | `next` | 15.x < 15.1.9 | `15.1.9` | RCE in React Server Components via crafted HTTP request (React2Shell / CVE-2025-55182) |
| CVE-2025-49826 | **High (CVSS 7.5)** | `next` | 15.1.0–15.1.7 | `15.1.9` | DoS via cache poisoning of HTTP 204 responses on static pages |

**Action taken:** `next` pinned to `15.1.9` (up from `15.1.0`). `eslint-config-next` aligned to `15.1.9` to match.

> **CVE-2025-66478 / CVE-2025-55182 detail:** An unauthenticated attacker can send a specially crafted HTTP request to any Next.js App Router application that uses React Server Components, leading to arbitrary code execution on the server. This affects all `next` versions in the 15.x line below `15.1.9`. Rated CVSS 10.0. Actively exploited in the wild.

> **CVE-2025-49826 detail:** A logic flaw in Next.js causes HTTP 204 (No Content) responses to be incorrectly cached for static pages. An attacker can trigger this to poison the cache and serve blank pages to all subsequent visitors (DoS). Affects `next` 15.1.0–15.1.7.

### 🗑️ Deprecated Package Removed

| Package | Version | Reason |
|---------|---------|--------|
| `critters` | `^0.0.23` | Officially deprecated by the Google Chrome team (October 2024). Next.js has migrated to the actively maintained fork `beasties`. The `critters` package is no longer needed as a standalone dependency — Next.js bundles `beasties` internally. Tracked in [vercel/next.js#72036](https://github.com/vercel/next.js/issues/72036). |

### ✅ Packages Verified Clean (No Known Vulnerabilities)

| Package | Pinned Version | Notes |
|---------|---------------|-------|
| `react` | `^19.0.0` → resolves `19.2.6` | React 19.2.6 includes the fix for CVE-2025-55182 (patched in 19.2.3+). Lock file confirmed. |
| `react-dom` | `^19.0.0` → resolves `19.2.6` | Same as `react`. Patched. |
| `react-is` | `^19.2.7` | No known advisories. |
| `recharts` | `^3.7.0` → resolves `3.8.1` | No known advisories. |
| `swr` | `^2.4.1` → resolves `2.4.2` | No known advisories. |
| `tailwind-merge` | `^2.3.0` → resolves `2.6.1` | No known advisories. |
| `@radix-ui/react-checkbox` | `^1.0.4` → resolves `1.3.3` | No known advisories. |
| `@radix-ui/react-slot` | `^1.0.2` → resolves `1.2.4` | No known advisories. |
| `@radix-ui/react-tooltip` | `^1.0.7` → resolves `1.2.8` | No known advisories. |
| `class-variance-authority` | `^0.7.0` → resolves `0.7.1` | No known advisories. |
| `clsx` | `^2.1.1` → resolves `2.1.1` | No known advisories. |
| `lucide-react` | `^0.563.0` → resolves `0.563.0` | No known advisories. |

### Dev Dependencies — Verified Clean

| Package | Notes |
|---------|-------|
| `eslint@^9.39.2` → `9.39.4` | No known advisories. |
| `jest@^29.7.0` | No known advisories. |
| `typescript@^5` → `5.9.3` | No known advisories. |
| `tailwindcss@^3.4.1` → `3.4.19` | No known advisories. |
| `autoprefixer@^10.4.19` → `10.5.0` | No known advisories. |
| `@testing-library/*` | No known advisories. |
| `ts-jest@^29.2.5` | No known advisories. |

---

## Pinned Versions Rationale

| Package | Specifier | Rationale |
|---------|-----------|-----------|
| `next` | `15.1.9` (exact) | **Security pin.** Minimum version that resolves CVE-2025-66478 and CVE-2025-49826. Exact pin (no `^`) prevents accidental resolution to a vulnerable patch. Update only after reviewing the Next.js changelog for the 15.1.x line. |
| `eslint-config-next` | `15.1.9` (exact) | Must match `next` exactly; mismatches produce peer-dependency errors and broken lint rules. |

---

## Issue #955 Audit Summary (June 2, 2026)

All dependencies listed in `package.json` at the time were actively used within the codebase. No unused libraries were identified for removal.

### Production Dependencies (from #955)

| Library | Usage | Reference Files |
| :--- | :--- | :--- |
| `@radix-ui/react-checkbox` | Checkbox UI primitive | `components/ui/checkbox.tsx` |
| `@radix-ui/react-slot` | Polymorphic component support (asChild) | `components/ui/button.tsx` |
| `@radix-ui/react-tooltip` | Tooltip UI primitive | `components/ui/tooltip.tsx` |
| `class-variance-authority` | CSS-in-JS variant management | `ui/button.tsx`, `ui/toast.tsx` |
| `clsx` | Conditional class joining | `lib/utils.ts` |
| `lucide-react` | Icon library (Menu, X, Loader, etc.) | Widespread (Main Nav, Buttons, Cards) |
| `next` | Core Framework | `app/`, `next.config.ts` |
| `react` | Core UI Library | Widespread |
| `react-dom` | Core UI Library (Web Support) | Widespread |
| `recharts` | Data visualization charts | `components/dashboard/StakedChart.tsx` |
| `tailwind-merge` | Smart Tailwind class conflict resolution | `lib/utils.ts` |
| `critters` | ~~CSS inlining for critical path~~ | **Removed in #1408 (deprecated)** |

### Development Dependencies (from #955)

| Library | Usage |
| :--- | :--- |
| `autoprefixer` | PostCSS plugin for browser prefixing in `postcss.config.mjs` |
| `eslint` & `eslint-config-next` | Linting configuration in `eslint.config.mjs` |
| `postcss` | CSS transformation engine |
| `tailwindcss` | Utility-first CSS framework |
| `typescript` | Static typing support |
| `@types/*` | Type definitions for Node, React, and DOM |

---

## Audit Methodology (#1408)

1. **CVE database cross-reference** — All production dependencies were checked against the GitHub Advisory Database (GHSA), the npm audit advisory feed, and NIST NVD for known vulnerabilities published since the previous audit.
2. **Lock-file version confirmation** — Exact resolved versions from `pnpm-lock.yaml` were used for vulnerability matching (not the semver specifiers in `package.json`).
3. **Deprecation scan** — Package metadata (npm registry, GitHub repository activity) was reviewed for deprecation notices.
4. **Transitive dependency review** — Indirect dependencies introduced by `next`, `recharts`, and `@radix-ui/*` were reviewed for any flagged advisories at the time of this audit.
5. **Remediation verification** — After bumping `next` to `15.1.9`, the patched versions of transitive React packages (`react-server-dom-webpack`, etc.) bundled inside Next.js were confirmed to include the CVE-2025-55182 fix.

---

## Recommended Follow-up Actions

1. **Run `pnpm audit` in CI** — Add `pnpm audit --audit-level=high` to the frontend CI workflow (`.github/workflows/frontend-ci.yml`) to catch future advisories automatically.
2. **Enable Dependabot** — Configure `.github/dependabot.yml` for the `frontend/` directory with `package-ecosystem: npm` and a weekly schedule.
3. **Monitor Next.js 15.x security releases** — The 15.x line has had multiple security releases in 2025. Subscribe to [github.com/vercel/next.js/security/advisories](https://github.com/vercel/next.js/security/advisories).

---

*Audit #955 Date: June 2, 2026*
*Audit #1408 Date: July 28, 2026*
