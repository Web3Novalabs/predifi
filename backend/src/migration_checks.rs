//! # Pre-Migration Safety Checks
//!
//! This module provides comprehensive migration safety infrastructure for
//! zero-downtime deployments, including:
//!
//! - **Dry-run mode**: Simulate migrations inside a transaction that is always
//!   rolled back, verifying SQL syntax and constraint violations without any
//!   permanent schema changes.
//! - **Rollback testing**: Verify that rolling back the dry-run transaction
//!   restores the schema to its prior state.
//! - **Schema diff preview**: Compute a human-readable diff of table/column
//!   changes that would be introduced by pending migrations.
//! - **Backward-compatibility checks**: Detect destructive DDL operations
//!   (column/table drops, type changes, NOT NULL additions) that would break
//!   running replicas before they are updated.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use predifi_backend::migration_checks::{MigrationChecker, CheckOptions};
//!
//! let checker = MigrationChecker::new(pool.clone());
//! let opts = CheckOptions { dry_run: true, check_backward_compat: true, ..Default::default() };
//! let report = checker.run(&opts).await?;
//! println!("{}", report.summary());
//! ```

use sqlx::{postgres::PgConnection, FromRow, PgPool};
use std::collections::{HashMap, HashSet};
use std::fmt;
use tracing::{info, warn};

// ── Data types ────────────────────────────────────────────────────────────────

/// A single column descriptor extracted from `information_schema`.
///
/// `FromRow` is derived so the runtime `sqlx::query_as::<_, ColumnDescriptor>()`
/// can deserialize rows without requiring `DATABASE_URL` at compile time.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ColumnDescriptor {
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
    pub ordinal_position: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct SchemaSnapshot {
    columns: HashMap<String, Vec<ColumnDescriptor>>,
    indexes: HashMap<String, HashSet<String>>,
}

/// Represents a single schema-change entry in the diff output.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDiffEntry {
    /// A table that does not yet exist will be created.
    TableAdded(String),
    /// A table that exists will be removed.
    TableDropped(String),
    /// An index that does not yet exist will be created.
    IndexAdded { table: String, index: String },
    /// An existing index will be removed.
    IndexDropped { table: String, index: String },
    /// A column that does not yet exist will be added.
    ColumnAdded {
        table: String,
        column: String,
        data_type: String,
        nullable: bool,
        has_default: bool,
    },
    /// An existing column will be removed.
    ColumnDropped { table: String, column: String },
    /// An existing column's default expression will change.
    DefaultChanged {
        table: String,
        column: String,
        from: Option<String>,
        to: Option<String>,
    },
    /// An existing column's type will change.
    TypeChanged {
        table: String,
        column: String,
        from: String,
        to: String,
    },
    /// A column's nullability will change.
    NullabilityChanged {
        table: String,
        column: String,
        was_nullable: bool,
        now_nullable: bool,
    },
}

impl fmt::Display for SchemaDiffEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TableAdded(t) => write!(f, "[ADD TABLE]    {t}"),
            Self::TableDropped(t) => write!(f, "[DROP TABLE]   {t}"),
            Self::IndexAdded { table, index } => write!(f, "[ADD INDEX]    {table}.{index}"),
            Self::IndexDropped { table, index } => write!(f, "[DROP INDEX]   {table}.{index}"),
            Self::ColumnAdded {
                table,
                column,
                data_type,
                nullable,
                has_default,
                ..
            } => write!(
                f,
                "[ADD COLUMN]   {table}.{column} {data_type}{}{}",
                if *nullable { "" } else { " NOT NULL" },
                if *has_default { " DEFAULT" } else { "" }
            ),
            Self::ColumnDropped { table, column } => {
                write!(f, "[DROP COLUMN]  {table}.{column}")
            }
            Self::DefaultChanged {
                table,
                column,
                from,
                to,
            } => write!(
                f,
                "[DEFAULT CHANGE] {table}.{column}: {} → {}",
                from.as_deref().unwrap_or("<none>"),
                to.as_deref().unwrap_or("<none>")
            ),
            Self::TypeChanged {
                table,
                column,
                from,
                to,
            } => write!(f, "[TYPE CHANGE]  {table}.{column}: {from} → {to}"),
            Self::NullabilityChanged {
                table,
                column,
                was_nullable,
                now_nullable,
            } => write!(
                f,
                "[NULL CHANGE]  {table}.{column}: {} → {}",
                if *was_nullable {
                    "nullable"
                } else {
                    "NOT NULL"
                },
                if *now_nullable {
                    "nullable"
                } else {
                    "NOT NULL"
                }
            ),
        }
    }
}

/// Classification of a schema change from a backward-compatibility perspective.
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityRisk {
    /// The change is safe for zero-downtime deployment.
    Safe,
    /// The change may break running replicas or clients.
    Breaking(String),
    /// The change requires a careful multi-step rollout.
    RequiresPhasing(String),
}

/// An individual backward-compatibility finding.
#[derive(Debug, Clone)]
pub struct CompatibilityFinding {
    pub entry: SchemaDiffEntry,
    pub risk: CompatibilityRisk,
}

/// Summary of a dry-run execution.
#[derive(Debug, Default)]
pub struct DryRunResult {
    /// Whether the migration SQL applied successfully inside the rolled-back txn.
    pub success: bool,
    /// Any SQL error encountered during the dry run.
    pub error: Option<String>,
    /// Number of migration files applied inside the dry-run transaction.
    pub statements_executed: usize,
    /// Whether rolling back the dry-run transaction restored the original schema.
    pub rollback_succeeded: bool,
}

/// The aggregated report produced by [`MigrationChecker::run`].
#[derive(Debug, Default)]
pub struct MigrationReport {
    /// Results of the dry-run phase (if `opts.dry_run` is true).
    pub dry_run: Option<DryRunResult>,
    /// Schema diff entries computed before and after applying migrations.
    pub schema_diff: Vec<SchemaDiffEntry>,
    /// Backward-compatibility findings.
    pub compat_findings: Vec<CompatibilityFinding>,
    /// Whether any non-safe change was detected.
    pub has_breaking_changes: bool,
    /// Total pending migration files detected.
    pub pending_migrations: usize,
}

impl MigrationReport {
    /// Return a formatted, multi-line summary of the report.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str("═══════════════════════════════════════════════════\n");
        out.push_str("          PrediFi Migration Safety Report\n");
        out.push_str("═══════════════════════════════════════════════════\n");

        out.push_str(&format!(
            "Pending migrations : {}\n",
            self.pending_migrations
        ));

        // Dry-run result
        if let Some(dry) = &self.dry_run {
            out.push_str(&format!(
                "Dry-run            : {}\n",
                if dry.success {
                    "✓ PASSED"
                } else {
                    "✗ FAILED"
                }
            ));
            if let Some(err) = &dry.error {
                out.push_str(&format!("  Error            : {err}\n"));
            }
            out.push_str(&format!(
                "  Migration files run: {}\n",
                dry.statements_executed
            ));
            out.push_str(&format!(
                "  Rollback check    : {}\n",
                if dry.rollback_succeeded {
                    "PASSED"
                } else {
                    "FAILED"
                }
            ));
        }

        // Schema diff
        if !self.schema_diff.is_empty() {
            out.push_str("\n─── Schema Diff ────────────────────────────────────\n");
            for entry in &self.schema_diff {
                out.push_str(&format!("  {entry}\n"));
            }
        } else if self.pending_migrations > 0 {
            out.push_str("\n─── Schema Diff ────────────────────────────────────\n");
            out.push_str("  No schema changes detected\n");
        }

        // Compatibility findings
        if !self.compat_findings.is_empty() {
            out.push_str("\n─── Backward Compatibility ─────────────────────────\n");
            for f in &self.compat_findings {
                let label = match &f.risk {
                    CompatibilityRisk::Safe => "✓ SAFE     ",
                    CompatibilityRisk::Breaking(_) => "✗ BREAKING ",
                    CompatibilityRisk::RequiresPhasing(_) => "⚠ PHASE-IN ",
                };
                out.push_str(&format!("  {label} {}\n", f.entry));
                if let CompatibilityRisk::Breaking(reason)
                | CompatibilityRisk::RequiresPhasing(reason) = &f.risk
                {
                    out.push_str(&format!("             → {reason}\n"));
                }
            }
        }

        out.push_str("\n─── Overall ────────────────────────────────────────\n");
        if self.has_breaking_changes {
            out.push_str("  ✗ UNSAFE CHANGES DETECTED — review before deploy\n");
        } else {
            out.push_str("  ✓ No unsafe changes detected\n");
        }
        out.push_str("═══════════════════════════════════════════════════\n");

        out
    }
}

// ── Options ───────────────────────────────────────────────────────────────────

/// Configuration for a [`MigrationChecker`] run.
#[derive(Debug, Clone)]
pub struct CheckOptions {
    /// Apply migrations inside a rolled-back transaction to validate SQL.
    pub dry_run: bool,
    /// Compute the schema diff and emit it in the report.
    pub show_diff: bool,
    /// Analyse the diff for backward-compatibility issues.
    pub check_backward_compat: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            dry_run: true,
            show_diff: true,
            check_backward_compat: true,
        }
    }
}

// ── MigrationChecker ──────────────────────────────────────────────────────────

/// Core migration safety checker.
pub struct MigrationChecker {
    pool: PgPool,
}

impl MigrationChecker {
    /// Create a new checker backed by the given pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Execute all requested checks and return a consolidated [`MigrationReport`].
    pub async fn run(&self, opts: &CheckOptions) -> Result<MigrationReport, sqlx::Error> {
        let mut report = MigrationReport::default();
        let migrator = sqlx::migrate!("./migrations");
        let applied = self.applied_migrations().await?;

        let mut versions = HashSet::new();
        for migration in migrator.iter() {
            if !versions.insert(migration.version) {
                report.dry_run = Some(DryRunResult {
                    success: false,
                    error: Some(format!(
                        "duplicate migration version {} ({})",
                        migration.version, migration.description
                    )),
                    statements_executed: 0,
                    rollback_succeeded: false,
                });
                return Ok(report);
            }
        }
        for version in applied.keys() {
            if !versions.contains(version) {
                report.dry_run = Some(DryRunResult {
                    success: false,
                    error: Some(format!(
                        "database contains migration version {version} that is missing from the repository"
                    )),
                    statements_executed: 0,
                    rollback_succeeded: false,
                });
                return Ok(report);
            }
        }

        let mut pending = Vec::new();
        for migration in migrator.iter() {
            match applied.get(&migration.version) {
                Some((false, _)) => {
                    report.dry_run = Some(DryRunResult {
                        success: false,
                        error: Some(format!(
                            "migration {} ({}) is recorded as failed",
                            migration.version, migration.description
                        )),
                        statements_executed: 0,
                        rollback_succeeded: false,
                    });
                    return Ok(report);
                }
                Some((true, checksum)) if checksum.as_slice() != migration.checksum.as_ref() => {
                    report.dry_run = Some(DryRunResult {
                        success: false,
                        error: Some(format!(
                            "checksum mismatch for applied migration {} ({})",
                            migration.version, migration.description
                        )),
                        statements_executed: 0,
                        rollback_succeeded: false,
                    });
                    return Ok(report);
                }
                Some((true, _)) => {}
                None => pending.push(migration),
            }
        }
        report.pending_migrations = pending.len();
        info!(
            pending = report.pending_migrations,
            "migration safety check started"
        );

        if opts.dry_run {
            let before = self.snapshot_schema().await?;
            let (dry, diff) = self.run_dry_run(&pending, Some(&before)).await;
            report.dry_run = Some(dry);
            if opts.show_diff || opts.check_backward_compat {
                report.schema_diff = diff;
            }
        } else if opts.show_diff || opts.check_backward_compat {
            report.dry_run = Some(DryRunResult {
                success: false,
                error: Some(
                    "dry-run is required to generate a schema preview and verify rollback"
                        .to_string(),
                ),
                statements_executed: 0,
                rollback_succeeded: false,
            });
            warn!("schema diff and compatibility checks require dry-run mode");
        }

        if opts.check_backward_compat {
            report.compat_findings = analyse_compatibility(&report.schema_diff);
            report.has_breaking_changes = report
                .compat_findings
                .iter()
                .any(|finding| finding.risk != CompatibilityRisk::Safe);
        }

        if report.has_breaking_changes {
            let unsafe_findings = report
                .compat_findings
                .iter()
                .filter(|finding| finding.risk != CompatibilityRisk::Safe)
                .count();
            warn!(
                unsafe_findings,
                "migration changes require review before zero-downtime deployment"
            );
        }

        Ok(report)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    async fn snapshot_schema(&self) -> Result<SchemaSnapshot, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        Self::snapshot_schema_on(&mut connection).await
    }

    async fn snapshot_schema_on(
        connection: &mut PgConnection,
    ) -> Result<SchemaSnapshot, sqlx::Error> {
        let rows = sqlx::query_as::<_, ColumnDescriptor>(
            r#"
            SELECT table_name, column_name, data_type,
                   (is_nullable = 'YES') AS is_nullable,
                   column_default, ordinal_position
            FROM information_schema.columns
            WHERE table_schema = 'public'
            ORDER BY table_name, ordinal_position
            "#,
        )
        .fetch_all(&mut *connection)
        .await?;

        let mut columns: HashMap<String, Vec<ColumnDescriptor>> = HashMap::new();
        for row in rows {
            columns.entry(row.table_name.clone()).or_default().push(row);
        }
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT tablename, indexname FROM pg_indexes \
             WHERE schemaname = 'public' ORDER BY tablename, indexname",
        )
        .fetch_all(&mut *connection)
        .await?;
        let mut indexes: HashMap<String, HashSet<String>> = HashMap::new();
        for (table, index) in rows {
            indexes.entry(table).or_default().insert(index);
        }
        Ok(SchemaSnapshot { columns, indexes })
    }

    async fn applied_migrations(&self) -> Result<HashMap<i64, (bool, Vec<u8>)>, sqlx::Error> {
        let exists: bool =
            sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        if !exists {
            return Ok(HashMap::new());
        }

        let rows: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(version, success, checksum)| (version, (success, checksum)))
            .collect())
    }

    /// Apply pending migrations in one transaction, capture the preview, and
    /// verify that rolling the transaction back restores the original schema.
    async fn run_dry_run(
        &self,
        pending: &[&sqlx::migrate::Migration],
        before: Option<&SchemaSnapshot>,
    ) -> (DryRunResult, Vec<SchemaDiffEntry>) {
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                return (
                    DryRunResult {
                        success: false,
                        error: Some(format!("failed to begin transaction: {e}")),
                        statements_executed: 0,
                        rollback_succeeded: false,
                    },
                    Vec::new(),
                );
            }
        };
        let mut count = 0usize;
        if let Err(error) =
            sqlx::raw_sql("SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '60s';")
                .execute(&mut *tx)
                .await
        {
            let rollback = tx.rollback().await;
            return (
                DryRunResult {
                    success: false,
                    error: Some(format!("failed to configure dry-run timeouts: {error}")),
                    statements_executed: count,
                    rollback_succeeded: rollback.is_ok(),
                },
                Vec::new(),
            );
        }
        for migration in pending {
            if migration.no_tx {
                let rollback = tx.rollback().await;
                return (
                    DryRunResult {
                        success: false,
                        error: Some(format!(
                            "migration {} ({}) cannot be dry-run because it disables transactions",
                            migration.version, migration.description
                        )),
                        statements_executed: count,
                        rollback_succeeded: rollback.is_ok(),
                    },
                    Vec::new(),
                );
            }

            count += 1;
            if let Err(error) = sqlx::raw_sql(migration.sql.as_ref())
                .execute(&mut *tx)
                .await
            {
                let rollback = tx.rollback().await;
                return (
                    DryRunResult {
                        success: false,
                        error: Some(format!(
                            "migration {} ({}) failed: {error}",
                            migration.version, migration.description
                        )),
                        statements_executed: count,
                        rollback_succeeded: rollback.is_ok(),
                    },
                    Vec::new(),
                );
            }
        }

        let diff = if let Some(before) = before {
            match Self::snapshot_schema_on(&mut tx).await {
                Ok(after) => compute_full_schema_diff(before, &after),
                Err(error) => {
                    let rollback = tx.rollback().await;
                    return (
                        DryRunResult {
                            success: false,
                            error: Some(format!("failed to snapshot migrated schema: {error}")),
                            statements_executed: count,
                            rollback_succeeded: rollback.is_ok(),
                        },
                        Vec::new(),
                    );
                }
            }
        } else {
            Vec::new()
        };

        if let Err(error) = tx.rollback().await {
            return (
                DryRunResult {
                    success: false,
                    error: Some(format!("failed to roll back dry-run transaction: {error}")),
                    statements_executed: count,
                    rollback_succeeded: false,
                },
                diff,
            );
        }

        let rollback_succeeded = match self.snapshot_schema().await {
            Ok(after_rollback) => before.is_none_or(|schema| schema == &after_rollback),
            Err(error) => {
                return (
                    DryRunResult {
                        success: false,
                        error: Some(format!("failed to verify rollback: {error}")),
                        statements_executed: count,
                        rollback_succeeded: false,
                    },
                    diff,
                )
            }
        };
        let error = (!rollback_succeeded)
            .then(|| "schema changed after the dry-run transaction was rolled back".to_string());
        info!(
            migrations = count,
            rollback_succeeded, "migration dry-run complete"
        );
        (
            DryRunResult {
                success: error.is_none(),
                error,
                statements_executed: count,
                rollback_succeeded,
            },
            diff,
        )
    }
}

// ── Schema diff computation ───────────────────────────────────────────────────

/// Compute the diff between a `before` and `after` schema snapshot.
pub fn compute_schema_diff(
    before: &HashMap<String, Vec<ColumnDescriptor>>,
    after: &HashMap<String, Vec<ColumnDescriptor>>,
) -> Vec<SchemaDiffEntry> {
    let mut diff = Vec::new();

    // Tables added
    for table in after.keys() {
        if !before.contains_key(table) {
            diff.push(SchemaDiffEntry::TableAdded(table.clone()));
        }
    }

    // Tables dropped
    for table in before.keys() {
        if !after.contains_key(table) {
            diff.push(SchemaDiffEntry::TableDropped(table.clone()));
        }
    }

    // Column-level diff for tables that exist in both snapshots
    for (table, after_cols) in after {
        if let Some(before_cols) = before.get(table) {
            let before_map: HashMap<&str, &ColumnDescriptor> = before_cols
                .iter()
                .map(|c| (c.column_name.as_str(), c))
                .collect();
            let after_map: HashMap<&str, &ColumnDescriptor> = after_cols
                .iter()
                .map(|c| (c.column_name.as_str(), c))
                .collect();

            // Columns added
            for (col_name, col) in &after_map {
                if !before_map.contains_key(col_name) {
                    diff.push(SchemaDiffEntry::ColumnAdded {
                        table: table.clone(),
                        column: col_name.to_string(),
                        data_type: col.data_type.clone(),
                        nullable: col.is_nullable,
                        has_default: col.column_default.is_some(),
                    });
                }
            }

            // Columns dropped / changed
            for (col_name, before_col) in &before_map {
                match after_map.get(col_name) {
                    None => diff.push(SchemaDiffEntry::ColumnDropped {
                        table: table.clone(),
                        column: col_name.to_string(),
                    }),
                    Some(after_col) => {
                        if before_col.data_type != after_col.data_type {
                            diff.push(SchemaDiffEntry::TypeChanged {
                                table: table.clone(),
                                column: col_name.to_string(),
                                from: before_col.data_type.clone(),
                                to: after_col.data_type.clone(),
                            });
                        }
                        if before_col.is_nullable != after_col.is_nullable {
                            diff.push(SchemaDiffEntry::NullabilityChanged {
                                table: table.clone(),
                                column: col_name.to_string(),
                                was_nullable: before_col.is_nullable,
                                now_nullable: after_col.is_nullable,
                            });
                        }
                        if before_col.column_default != after_col.column_default {
                            diff.push(SchemaDiffEntry::DefaultChanged {
                                table: table.clone(),
                                column: col_name.to_string(),
                                from: before_col.column_default.clone(),
                                to: after_col.column_default.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    diff.sort_by(|a, b| format!("{a}").cmp(&format!("{b}")));
    diff
}

fn compute_full_schema_diff(
    before: &SchemaSnapshot,
    after: &SchemaSnapshot,
) -> Vec<SchemaDiffEntry> {
    let mut diff = compute_schema_diff(&before.columns, &after.columns);
    let empty = HashSet::new();
    for (table, indexes) in &after.indexes {
        for index in indexes.difference(before.indexes.get(table).unwrap_or(&empty)) {
            diff.push(SchemaDiffEntry::IndexAdded {
                table: table.clone(),
                index: index.clone(),
            });
        }
    }
    for (table, indexes) in &before.indexes {
        for index in indexes.difference(after.indexes.get(table).unwrap_or(&empty)) {
            diff.push(SchemaDiffEntry::IndexDropped {
                table: table.clone(),
                index: index.clone(),
            });
        }
    }
    diff.sort_by_key(ToString::to_string);
    diff
}

// ── Backward-compatibility analysis ──────────────────────────────────────────

/// Classify each diff entry as safe, breaking, or phase-in required.
pub fn analyse_compatibility(diff: &[SchemaDiffEntry]) -> Vec<CompatibilityFinding> {
    diff.iter()
        .map(|entry| {
            let risk = classify_risk(entry);
            CompatibilityFinding {
                entry: entry.clone(),
                risk,
            }
        })
        .collect()
}

fn classify_risk(entry: &SchemaDiffEntry) -> CompatibilityRisk {
    match entry {
        SchemaDiffEntry::IndexAdded { .. } | SchemaDiffEntry::IndexDropped { .. } => {
            CompatibilityRisk::Safe
        }

        // Adding a new table is always safe.
        SchemaDiffEntry::TableAdded(_) => CompatibilityRisk::Safe,

        // Dropping a table is always breaking.
        SchemaDiffEntry::TableDropped(t) => CompatibilityRisk::Breaking(format!(
            "Dropping table '{t}' will break any code that still references it. \
             Use a multi-step rollout: deprecate → stop references → drop."
        )),

        // Adding a nullable column with a default is safe for zero-downtime.
        SchemaDiffEntry::ColumnAdded {
            nullable,
            has_default,
            table,
            column,
            ..
        } => {
            if *nullable || *has_default {
                CompatibilityRisk::Safe
            } else {
                CompatibilityRisk::RequiresPhasing(format!(
                    "Adding NOT NULL column '{table}.{column}' without a default will \
                     fail on rows inserted by the old binary. Add a default or backfill first."
                ))
            }
        }

        // Dropping a column breaks old code that still SELECT/INSERT it.
        SchemaDiffEntry::ColumnDropped { table, column } => CompatibilityRisk::Breaking(format!(
            "Dropping '{table}.{column}' will break old binaries that reference it. \
                 Deprecate the column in code first."
        )),

        SchemaDiffEntry::DefaultChanged {
            table,
            column,
            from,
            to,
        } => {
            if from.is_none() && to.is_some() {
                CompatibilityRisk::Safe
            } else {
                CompatibilityRisk::RequiresPhasing(format!(
                    "Changing the default for '{table}.{column}' may alter old-code inserts; \
                     verify insert behavior before rollout."
                ))
            }
        }

        // Type changes are almost always breaking.
        SchemaDiffEntry::TypeChanged {
            table,
            column,
            from,
            to,
        } => CompatibilityRisk::Breaking(format!(
            "Changing '{table}.{column}' from {from} to {to} may corrupt data or \
                 break deserialization in running replicas."
        )),

        // Making a nullable column NOT NULL breaks old inserts that omit it.
        SchemaDiffEntry::NullabilityChanged {
            table,
            column,
            was_nullable,
            now_nullable,
        } => {
            if *was_nullable && !*now_nullable {
                CompatibilityRisk::Breaking(format!(
                    "Adding NOT NULL constraint to '{table}.{column}' will reject inserts \
                     from old code that does not supply the column."
                ))
            } else {
                // NOT NULL → nullable: safe to do online.
                CompatibilityRisk::Safe
            }
        }
    }
}

// ── Rollback test helpers ─────────────────────────────────────────────────────

/// Test that applying a migration and then rolling back its transaction
/// leaves the table and column schema unchanged.
pub async fn run_rollback_test(pool: &PgPool, migration_sql: &str) -> Result<bool, sqlx::Error> {
    let checker = MigrationChecker::new(pool.clone());
    let before = checker.snapshot_schema().await?;

    let mut tx = pool.begin().await?;
    if let Err(error) = sqlx::raw_sql(migration_sql).execute(&mut *tx).await {
        tx.rollback().await?;
        return Err(error);
    }
    tx.rollback().await?;

    let after = checker.snapshot_schema().await?;
    let unchanged = before == after;

    if unchanged {
        info!("rollback test passed: schema is identical before and after rollback");
    } else {
        warn!("rollback test FAILED: schema changed despite transaction rollback");
    }

    Ok(unchanged)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn col(table: &str, column: &str, data_type: &str, nullable: bool) -> ColumnDescriptor {
        ColumnDescriptor {
            table_name: table.to_string(),
            column_name: column.to_string(),
            data_type: data_type.to_string(),
            is_nullable: nullable,
            column_default: None,
            ordinal_position: 1,
        }
    }

    #[test]
    fn diff_detects_new_table() {
        let before: HashMap<String, Vec<ColumnDescriptor>> = HashMap::new();
        let mut after = HashMap::new();
        after.insert(
            "pools".to_string(),
            vec![col("pools", "id", "bigint", false)],
        );

        let diff = compute_schema_diff(&before, &after);
        assert!(diff
            .iter()
            .any(|e| matches!(e, SchemaDiffEntry::TableAdded(t) if t == "pools")));
    }

    #[test]
    fn diff_detects_dropped_table() {
        let mut before = HashMap::new();
        before.insert(
            "old_table".to_string(),
            vec![col("old_table", "id", "bigint", false)],
        );
        let after: HashMap<String, Vec<ColumnDescriptor>> = HashMap::new();

        let diff = compute_schema_diff(&before, &after);
        assert!(diff
            .iter()
            .any(|e| matches!(e, SchemaDiffEntry::TableDropped(t) if t == "old_table")));
    }

    #[test]
    fn diff_detects_added_column() {
        let mut before = HashMap::new();
        before.insert(
            "pools".to_string(),
            vec![col("pools", "id", "bigint", false)],
        );
        let mut after = HashMap::new();
        after.insert(
            "pools".to_string(),
            vec![
                col("pools", "id", "bigint", false),
                col("pools", "name", "text", true),
            ],
        );

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|e| matches!(
            e,
            SchemaDiffEntry::ColumnAdded { table, column, .. }
            if table == "pools" && column == "name"
        )));
    }

    #[test]
    fn diff_detects_dropped_column() {
        let mut before = HashMap::new();
        before.insert(
            "pools".to_string(),
            vec![
                col("pools", "id", "bigint", false),
                col("pools", "legacy", "text", true),
            ],
        );
        let mut after = HashMap::new();
        after.insert(
            "pools".to_string(),
            vec![col("pools", "id", "bigint", false)],
        );

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|e| matches!(
            e,
            SchemaDiffEntry::ColumnDropped { table, column }
            if table == "pools" && column == "legacy"
        )));
    }

    #[test]
    fn diff_detects_type_change() {
        let mut before = HashMap::new();
        before.insert(
            "pools".to_string(),
            vec![col("pools", "amount", "integer", false)],
        );
        let mut after = HashMap::new();
        after.insert(
            "pools".to_string(),
            vec![col("pools", "amount", "bigint", false)],
        );

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|e| matches!(
            e,
            SchemaDiffEntry::TypeChanged { table, column, from, to }
            if table == "pools" && column == "amount" && from == "integer" && to == "bigint"
        )));
    }

    #[test]
    fn diff_detects_changed_column_default() {
        let mut before = HashMap::new();
        let mut old_column = col("pools", "state", "text", false);
        old_column.column_default = Some("'active'".to_string());
        before.insert("pools".to_string(), vec![old_column]);
        let mut after = HashMap::new();
        let mut new_column = col("pools", "state", "text", false);
        new_column.column_default = Some("'closed'".to_string());
        after.insert("pools".to_string(), vec![new_column]);

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|entry| matches!(
            entry,
            SchemaDiffEntry::DefaultChanged { table, column, .. }
            if table == "pools" && column == "state"
        )));
    }

    #[test]
    fn diff_detects_added_index() {
        let before = SchemaSnapshot::default();
        let mut after = SchemaSnapshot::default();
        after
            .indexes
            .entry("pools".to_string())
            .or_default()
            .insert("idx_pools_state".to_string());

        let diff = compute_full_schema_diff(&before, &after);
        assert!(diff.iter().any(|entry| matches!(
            entry,
            SchemaDiffEntry::IndexAdded { table, index }
            if table == "pools" && index == "idx_pools_state"
        )));
    }

    #[test]
    fn compat_table_drop_is_breaking() {
        let entry = SchemaDiffEntry::TableDropped("pools".to_string());
        let finding = &analyse_compatibility(&[entry])[0];
        assert!(matches!(finding.risk, CompatibilityRisk::Breaking(_)));
    }

    #[test]
    fn compat_nullable_column_add_is_safe() {
        let entry = SchemaDiffEntry::ColumnAdded {
            table: "pools".to_string(),
            column: "notes".to_string(),
            data_type: "text".to_string(),
            nullable: true,
            has_default: false,
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert_eq!(finding.risk, CompatibilityRisk::Safe);
    }

    #[test]
    fn compat_not_null_column_add_requires_phasing() {
        let entry = SchemaDiffEntry::ColumnAdded {
            table: "pools".to_string(),
            column: "required_field".to_string(),
            data_type: "text".to_string(),
            nullable: false,
            has_default: false,
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert!(matches!(
            finding.risk,
            CompatibilityRisk::RequiresPhasing(_)
        ));
    }

    #[test]
    fn compat_type_change_is_breaking() {
        let entry = SchemaDiffEntry::TypeChanged {
            table: "pools".to_string(),
            column: "amount".to_string(),
            from: "integer".to_string(),
            to: "text".to_string(),
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert!(matches!(finding.risk, CompatibilityRisk::Breaking(_)));
    }

    #[test]
    fn compat_nullable_to_not_null_is_breaking() {
        let entry = SchemaDiffEntry::NullabilityChanged {
            table: "pools".to_string(),
            column: "email".to_string(),
            was_nullable: true,
            now_nullable: false,
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert!(matches!(finding.risk, CompatibilityRisk::Breaking(_)));
    }

    #[test]
    fn compat_not_null_to_nullable_is_safe() {
        let entry = SchemaDiffEntry::NullabilityChanged {
            table: "pools".to_string(),
            column: "email".to_string(),
            was_nullable: false,
            now_nullable: true,
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert_eq!(finding.risk, CompatibilityRisk::Safe);
    }

    #[test]
    fn report_summary_contains_expected_sections() {
        let mut report = MigrationReport::default();
        report.pending_migrations = 2;
        report.dry_run = Some(DryRunResult {
            success: true,
            error: None,
            statements_executed: 5,
            rollback_succeeded: true,
        });
        report.schema_diff = vec![SchemaDiffEntry::TableAdded("new_table".to_string())];
        report.has_breaking_changes = false;

        let summary = report.summary();
        assert!(summary.contains("Pending migrations"));
        assert!(summary.contains("Dry-run"));
        assert!(summary.contains("new_table"));
        assert!(summary.contains("No unsafe changes"));
    }

    #[test]
    fn migrations_have_unique_versions() {
        let migrator = sqlx::migrate!("./migrations");
        let mut versions = HashSet::new();
        for migration in migrator.iter() {
            assert!(
                versions.insert(migration.version),
                "duplicate migration version {} ({})",
                migration.version,
                migration.description
            );
        }
    }

    #[test]
    fn compat_not_null_column_add_with_default_is_safe() {
        let entry = SchemaDiffEntry::ColumnAdded {
            table: "pools".to_string(),
            column: "state".to_string(),
            data_type: "text".to_string(),
            nullable: false,
            has_default: true,
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert_eq!(finding.risk, CompatibilityRisk::Safe);
    }
}
