# Database migrations

This directory holds the SQL migrations for the PrediFi backend's PostgreSQL
schema, run via [`sqlx migrate`](https://docs.rs/sqlx/latest/sqlx/migrate/index.html).
They are also embedded at compile time via `sqlx::migrate!("./migrations")`
and applied automatically by the server on startup, the `predifi-seed` binary,
and the integration test suite (`src/db_integration_tests.rs`).

## Naming convention

Each file is named:

```text
<NNN>_<description>.sql
```

- `<NNN>` is a zero-padded, monotonically increasing integer prefix (`001`,
  `002`, `003`, ...). sqlx parses the leading digits before the first `_` as
  the migration's numeric version and applies migrations in ascending version
  order.
- `<description>` is a short, snake_case summary of what the migration does.
- Migrations in this project are plain "up" scripts (no matching `.down.sql`
  file) — there is no supported rollback path, so undoing a change requires a
  new forward migration.

When adding a new migration, use the **next unused integer**, run
`sqlx migrate add <description>` (or create the file by hand) inside this
directory, and never reuse or renumber a prefix that already exists — see
[Duplicate prefixes](#duplicate-004-and-009-prefixes-do-not-rename) below.
The next available prefix is **`011_`**.

## Applying migrations locally

1. Make sure PostgreSQL is running and reachable (see the backend's main
   [README](../README.md#environment-configuration) for `DATABASE_URL`/
   `PREDIFI_DATABASE_URL` defaults).
2. Install the sqlx CLI once, if you don't already have it:

   ```bash
   cargo install sqlx-cli --no-default-features --features postgres,rustls
   ```

3. From the `backend/` directory, apply all pending migrations:

   ```bash
   cd backend
   DATABASE_URL=postgres://postgres:postgres@localhost:5432/predifi \
     sqlx migrate run --source ./migrations
   ```

   Use the same connection string as your `PREDIFI_DATABASE_URL` — sqlx-cli
   reads the plain `DATABASE_URL` variable, not the `PREDIFI_`-prefixed one
   the application itself uses.

The server, the `predifi-seed` binary, and the integration tests all call
`sqlx::migrate!("./migrations").run(&pool)` automatically before doing
anything else, so you normally don't need to run `sqlx migrate run` by hand —
**except** that, as of this writing, running the full migrator against a
truly empty/brand-new database currently fails outright with a duplicate-key
error, because of the duplicate version prefixes described below. See
[Duplicate prefixes](#duplicate-004-and-009-prefixes-do-not-rename) for
details before assuming a fresh `sqlx migrate run` (or a fresh
`predifi-seed` run, or the server's own startup migration) will succeed
against a database that has never had any migrations applied to it.

## Migration files

| File | Adds |
| :--- | :--- |
| `001_create_referrals.sql` | `referrals` table indexing on-chain referral events (a user who staked via a referrer link). |
| `002_create_pools.sql` | `pools` table indexing on-chain prediction pool data from the Stellar/Soroban contract. |
| `003_create_predictions.sql` | `predictions` table indexing individual user stakes on a pool outcome. |
| `004_creator_incentives_templates_and_indexes.sql` | Creator incentive columns + `creator_stats` on pools (#1366), pool templates for recurring markets (#1368), and extra lookup/sort indexes (#1369/#1370). |
| `004_enhance_schema_precision.sql` | Converts stake/amount columns to `NUMERIC(32, 7)`, adds `contract_id` to `pools`, creates a `stats` table, and adds performance indexes (#706). |
| `005_add_index_pools_created_at.sql` | Index on `pools.created_at` to speed up sorting by creation time. |
| `006_optimize_user_address_indexes.sql` | Indexes on user-address columns for prediction-history and referral lookups. |
| `007_optimize_pool_id_indexes.sql` | Indexes on `pool_id` columns for referral earnings and pool-scoped analytics. |
| `008_referrer_stats.sql` | Pre-aggregated `referrer_stats`, maintained by trigger, to avoid full-table scans on referral volume reads. |
| `009_add_predictions_indexes.sql` | Targeted indexes on `predictions` for query patterns not yet covered by an index. |
| `009_create_user_statistics.sql` | Pre-aggregated `user_statistics` table, maintained by triggers, for user betting volume/winnings/prediction counts. |
| `010_tags_claims_notifications.sql` | Multi-tag support on pools, claim tracking, and a notification system. |

All 12 files above exist in this directory today.

## Duplicate 004 and 009 prefixes (do not rename)

There are two files each starting with `004_` and two starting with `009_`:

- `004_creator_incentives_templates_and_indexes.sql` and
  `004_enhance_schema_precision.sql`
- `009_add_predictions_indexes.sql` and `009_create_user_statistics.sql`

This happened because both migrations in each pair were added independently
(different PRs/issues) without noticing the prefix was already taken.

**These files are intentionally left as-is and must not be renamed.** sqlx
records each applied migration by its parsed integer version in the
`_sqlx_migrations` table, keyed on that version. Any database that has
already run one (or both) of a duplicated pair has that version number
permanently recorded against the exact file content it applied. Renaming a
migration to a new prefix, or editing an already-applied file, changes its
checksum/identity from sqlx's point of view and will make sqlx believe the
migration was never applied (or was tampered with), breaking `sqlx migrate
run` on every existing database — local, staging, and production alike.

**Ordering risk:** sqlx resolves migrations by reading this directory
(`std::fs::read_dir`) and then sorting the results by version number only.
For two files that share the same version number, their relative order after
that sort depends on the order the filesystem happened to return them in,
which is not guaranteed by any OS to match alphabetical/lexical filename
order — you cannot assume `004_creator_incentives_templates_and_indexes.sql`
runs before or after `004_enhance_schema_precision.sql` (and likewise for the
`009_` pair). Never add a migration that depends on a specific ordering
between two files that share a prefix.

In practice this goes beyond an ordering ambiguity: `_sqlx_migrations` tracks
applied migrations with `version` as its **primary key**, so running the
migrator against a genuinely empty database currently fails outright. Whichever
of the two same-numbered files sqlx applies first succeeds and is recorded
under that version; the second one it reaches then fails with `duplicate key
value violates unique constraint "_sqlx_migrations_pkey"`, since sqlx tries to
insert a second row for a version that's already taken. This was confirmed by
running `sqlx::migrate!("./migrations").run(&pool)` against a brand-new
Postgres database — both the `004_` pair and the `009_` pair hit this. It
means the server, `predifi-seed`, and `sqlx migrate run` can all currently
fail to bootstrap a database that has never had any migrations applied to it
(see [Applying migrations locally](#applying-migrations-locally)). Existing
databases that already have one migration of each pair recorded are
unaffected by this — the failure only shows up on a first run from empty.
Fixing it for real would mean consolidating or renumbering one file in each
pair, which is exactly what would break the checksum/version history on
databases that already ran it — hence this issue being documentation-only.

**Convention going forward:** always use the next unused integer prefix, and
double-check this table (or `ls backend/migrations/`) before naming a new
migration file so prefixes are never reused again. The next migration should
be named `011_<description>.sql`.
