# OpenAPI spec validation & TypeScript client generation

Covers [#1381](https://github.com/Web3Novalabs/predifi/issues/1381).

The backend documents its HTTP API by hand in `backend/src/openapi.rs`. Because
those annotations sit on stub functions rather than on the handlers themselves,
nothing stops the spec from drifting away from the router. Two pieces keep it
honest: integration tests that check the spec against the live router, and a
generator that turns the spec into frontend types.

## 1. Spec validation

`backend/src/openapi_tests.rs` builds the real Axum router (via
`build_router`) and asserts:

| Test | What it catches |
| --- | --- |
| `documented_paths_are_routed` | A documented endpoint that the router does not serve (404/405) |
| `every_schema_ref_resolves` | A `$ref` pointing at a schema that was never registered in `components(schemas(...))` |
| `documented_operations_declare_responses` | An operation with no responses, or no 2xx response — generators emit `unknown` bodies for these |
| `spec_is_served_as_valid_json` | `/api-docs/openapi.json` not serving, or serving something other than the compiled-in `ApiDoc` |

Run them with:

```bash
cargo test --manifest-path backend/Cargo.toml openapi
```

A handler returning 500/503 without a database is fine — the tests assert the
route *exists*, not that it succeeds.

## 2. Exporting the spec

```bash
cargo run --manifest-path backend/Cargo.toml --bin predifi-openapi -- --out openapi.json
```

No server, database, or Redis required — the spec is compiled in. Omit `--out`
to print to stdout.

## 3. Generating TypeScript types

```bash
cd frontend && pnpm generate:api
```

This exports the spec and runs [`openapi-typescript`](https://openapi-ts.dev)
over it, writing `frontend/types/api.d.ts`. Then import the generated types
instead of hand-writing response shapes:

```ts
import type { components } from "@/types/api";

type Pool = components["schemas"]["PoolDoc"];
type PoolList = components["schemas"]["PoolListResponse"];

async function fetchPools(): Promise<PoolList> {
  const res = await fetch("/api/v1/pools");
  return res.json();
}
```

Overrides: `OUT=./types/backend.d.ts pnpm generate:api` changes the output
path, `SPEC=./openapi.json pnpm generate:api` reuses an existing spec file
(useful in CI, where the spec can be exported once and shared).

## Keeping it in sync

When you add or change an endpoint:

1. Add the `#[utoipa::path(...)]` stub in `backend/src/openapi.rs`, and register
   any new schema in `components(schemas(...))`.
2. Run the spec tests — they fail if the stub and the router disagree.
3. Re-run `pnpm generate:api` and commit the regenerated types.
