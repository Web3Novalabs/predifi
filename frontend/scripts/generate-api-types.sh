#!/usr/bin/env bash
# Generate TypeScript types from the backend OpenAPI spec (#1381).
#
# Exports the spec straight from the Rust source of truth (no running server
# needed) and converts it to types the frontend API layer can import, so a
# backend contract change surfaces as a TypeScript error rather than a runtime
# surprise.
#
# Usage:
#   pnpm generate:api                       # export spec, then generate types
#   SPEC=./openapi.json pnpm generate:api   # reuse an existing spec file
set -euo pipefail

FRONTEND_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_DIR="$(cd "${FRONTEND_DIR}/../backend" && pwd)"
OUT="${OUT:-${FRONTEND_DIR}/types/api.d.ts}"
SPEC="${SPEC:-}"

if [[ -z "${SPEC}" ]]; then
  SPEC="$(mktemp -t predifi-openapi.XXXXXX).json"
  echo "→ exporting OpenAPI spec from ${BACKEND_DIR}"
  (cd "${BACKEND_DIR}" && cargo run --quiet --bin predifi-openapi -- --out "${SPEC}")
fi

echo "→ generating ${OUT}"
mkdir -p "$(dirname "${OUT}")"
npx --yes openapi-typescript@7 "${SPEC}" --output "${OUT}"

echo "✓ API types written to ${OUT}"
echo "  import type { components, paths } from '@/types/api';"
