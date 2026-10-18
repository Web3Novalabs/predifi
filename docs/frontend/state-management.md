# Frontend State Management

Audit, consolidation, and data-flow rules for PrediFi's Next.js frontend
(#1973).

## Audit summary

| Kind of state | Where it lives | Notes |
| ------------- | -------------- | ----- |
| Server data (pools, profile, tags, notifications) | SWR cache via `lib/hooks/*.ts` | Single source of truth; keys are request URLs |
| Wallet/session | `lib/context/WalletContext` | Global context, not per-component state |
| Theme | `lib/context/ThemeContext` | Global context |
| URL-synced filters (search, page, sort) | `lib/hooks/usePoolsQuery` | Derived from the URL — no duplicate local copies |
| Ephemeral UI state (open/closed, input drafts, toasts) | Local `useState` in the component that owns it | Must not be mirrored elsewhere |
| Optimistic writes | `lib/hooks/useOptimisticAction` | Reconciled with SWR on success/rollback |

**Rule:** if a component needs data another component already fetches, it must
read the shared SWR hook — never refetch into its own `useState`/`useEffect`.

## Cache invalidation

All invalidation goes through `lib/cache.ts`:

```ts
import { invalidateCache } from "@/lib/cache";

await invalidateCache("notifications"); // one group
await invalidateCache(["pools", "user"]); // several groups
await invalidateCache(); // everything
```

Groups mirror the API routes (`pools`, `notifications`, `user`, `tags`), so a
mutation invalidates exactly the dependent entries instead of scattering
`mutate()` calls across components. Prefer `invalidateCache` over a hook's
local `mutate` whenever a write affects more than one entry; per-entry
`mutate` is fine for "retry this one request" affordances.

After a write: **mutate → invalidate the owning group → update UI state.**
The `await` on `invalidateCache` resolves once the refetch settles, so
"Saved" indicators only show once the cache is fresh.

## Re-render rules

- Do not allocate default array/object literals inline in props or destructuring
  defaults (`data: items = []`) — each render creates a new identity and re-fires
  `useEffect` deps. Use a module-level constant (`EMPTY_INTERESTS`) or `useMemo`.
- Keep derived values derived: compute in render or `useMemo`, never in state
  synced with `useEffect`.
- Expensive lists virtualize or memoize (`memo`, `useMemo`) on the actual
  dependency, not on unstable callbacks — wrap handlers in `useCallback`.
- Fetch only what the view renders; paginate server-side (`limit`/`offset`)
  rather than filtering a full client-side copy.

## Data-flow pattern

```
URL (filters) ──▶ usePoolsQuery ──▶ usePools (SWR) ──▶ components
                                   ▲
mutation (optimistic) ──▶ invalidateCache(group) ──┘
```

Unidirectional: components render from hooks, mutations write through the
API layer, and freshness is restored centrally via `invalidateCache`.
