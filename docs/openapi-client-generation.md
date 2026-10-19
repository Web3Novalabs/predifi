# OpenAPI contract validation and TypeScript types

The backend API contract is defined in `backend/src/openapi.rs`. Integration
tests exercise the real Axum router and compare representative response bodies
with their documented OpenAPI schemas. The frontend generates TypeScript types
from that same spec and uses generated types in its pool API client.

## Validate the contract

Run the backend contract tests with:

```bash
cargo test --manifest-path backend/Cargo.toml openapi
```

The tests verify that documented paths are routed, every schema reference
resolves, operations declare success responses, the spec endpoint serves the
compiled spec, and representative runtime responses match their documented
status and JSON schema. The test router has no database connection, so pool
operations exercise their documented dependency-error responses; `/fees`
exercises a successful response.

## Generate frontend types

Install frontend dependencies and run:

```bash
cd frontend
pnpm install
pnpm generate:api
```

The script exports the spec from the Rust source (no server, database, or Redis
needed) and writes `frontend/types/api.d.ts`. The generated file is used by
`frontend/lib/api/pools.ts` for response types. The pool-list endpoint retains
its direct success body, while pool details use the documented API envelope;
the client normalizes date-time strings to Unix seconds for existing UI
consumers.

To export the spec separately:

```bash
cargo run --manifest-path backend/Cargo.toml --bin predifi-openapi -- --out backend/openapi.json
```

The generator also accepts `SPEC=./openapi.json` to reuse an exported spec and
`OUT=./types/backend.d.ts` to select a different output path.

To verify that the committed `openapi.json` is up to date:

```bash
cargo run --manifest-path backend/Cargo.toml --bin predifi-openapi -- --check backend/openapi.json
```

This verification is run automatically in CI on pull requests and pushes affecting `backend/**`.

## CI and contract updates

Frontend CI installs Rust and Node dependencies, generates API types, and runs
`pnpm typecheck` before linting, tests, and the production build. A backend
OpenAPI schema change that breaks the pool client therefore fails frontend
typechecking.

Use generated types directly in client code:

```ts
import type { components } from "@/types/api";

type Pool = components["schemas"]["PoolDoc"];
type PoolList = components["schemas"]["PoolListResponse"];
```

When changing an endpoint:

1. Update its `#[utoipa::path(...)]` documentation and registered schemas in
   `backend/src/openapi.rs`.
2. Update or add runtime schema coverage in `backend/src/openapi_tests.rs`.
3. Run the OpenAPI tests, then `cd frontend && pnpm generate:api && pnpm typecheck`.
4. Regenerate and commit `backend/openapi.json` and the generated frontend types
   with the contract changes.
