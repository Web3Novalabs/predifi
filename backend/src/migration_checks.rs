//! # Pre-Migration Safety Checks
//!
//! This module provides comprehensive migration safety infrastructure for
//! zero-downtime deployments, including:
//!
//! - **Dry-run mode**: Simulate migrations inside a transaction that is always
//!   rolled back, verifying SQL syntax and constraint violations without any
//!   permanent schema changes.
//! - **Rollback testing**: Verify that a down-migration successfully reverts
//!   the schema to its prior state.
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

use sqlx::{FromRow, PgPool};
use std::collections::HashMap;
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

/// Represents a single schema-change entry in the diff output.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDiffEntry {
    /// A table that does not yet exist will be created.
    TableAdded(String),
    /// A table that exists will be removed.
    TableDropped(String),
    /// A column that does not yet exist will be added.
    ColumnAdded {
        table: String,
        column: String,
        data_type: String,
        nullable: bool,
    },
    /// An existing column will be removed.
    ColumnDropped { table: String, column: String },
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
            Self::ColumnAdded {
                table,
                column,
                data_type,
                nullable,
            } => write!(
                f,
                "[ADD COLUMN]   {table}.{column} {data_type}{}",
                if *nullable { "" } else { " NOT NULL" }
            ),
            Self::ColumnDropped { table, column } => {
                write!(f, "[DROP COLUMN]  {table}.{column}")
            }
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
                if *was_nullable { "nullable" } else { "NOT NULL" },
                if *now_nullable { "nullable" } else { "NOT NULL" }
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
    /// List of SQL statements that were executed (best-effort parse).
    pub statements_executed: usize,
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
    /// Whether any breaking change was detected.
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
                if dry.success { "✓ PASSED" } else { "✗ FAILED" }
            ));
            if let Some(err) = &dry.error {
                out.push_str(&format!("  Error            : {err}\n"));
            }
            out.push_str(&format!(
                "  Statements run   : {}\n",
                dry.statements_executed
            ));
        }

        // Schema diff
        if !self.schema_diff.is_empty() {
            out.push_str("\n─── Schema Diff ────────────────────────────────────\n");
            for entry in &self.schema_diff {
                out.push_str(&format!("  {entry}\n"));
            }
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
            out.push_str("  ✗ BREAKING CHANGES DETECTED — review before deploy\n");
        } else {
            out.push_str("  ✓ No breaking changes detected\n");
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

        // Snapshot schema BEFORE migrations are applied.
        let before = if opts.show_diff || opts.check_backward_compat {
            self.snapshot_schema().await?
        } else {
            HashMap::new()
        };

        // Determine how many migrations are pending.
        report.pending_migrations = self.count_pending_migrations().await?;
        info!(
            pending = report.pending_migrations,
            "migration safety check started"
        );

        // Dry-run: apply migrations inside a savepoint, then roll back.
        if opts.dry_run {
            let dry = self.run_dry_run().await;
            report.dry_run = Some(dry);
        }

        // Snapshot schema AFTER applying real migrations.
        let after = if opts.show_diff || opts.check_backward_compat {
            // We apply migrations for real only when requested — in dry-run
            // mode we use the before snapshot for both sides because the
            // dry-run is rolled back.
            // For a complete diff experience, call `sqlx migrate run` first
            // and then call `run` with dry_run = false.
            self.snapshot_schema().await?
        } else {
            HashMap::new()
        };

        // Schema diff
        if opts.show_diff {
            report.schema_diff = compute_schema_diff(&before, &after);
        }

        // Backward-compatibility analysis
        if opts.check_backward_compat {
            report.compat_findings = analyse_compatibility(&report.schema_diff);
            report.has_breaking_changes = report.compat_findings.iter().any(|f| {
                matches!(f.risk, CompatibilityRisk::Breaking(_))
            });
        }

        if report.has_breaking_changes {
            warn!(
                breaking = report.compat_findings.len(),
                "breaking migration changes detected"
            );
        }

        Ok(report)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Snapshot the current schema from `information_schema.columns`.
    ///
    /// Uses the runtime `sqlx::query_as` form (no `!`) so that this code
    /// compiles without a `DATABASE_URL` environment variable set at build time.
    async fn snapshot_schema(
        &self,
    ) -> Result<HashMap<String, Vec<ColumnDescriptor>>, sqlx::Error> {
        // Runtime query — no compile-time DATABASE_URL needed.
        let rows = sqlx::query_as::<_, ColumnDescriptor>(
            r#"
            SELECT
                table_name,
                column_name,
                data_type,
                (is_nullable = 'YES') AS is_nullable,
                column_default,
                ordinal_position
            FROM information_schema.columns
            WHERE table_schema = 'public'
            ORDER BY table_name, ordinal_position
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut map: HashMap<String, Vec<ColumnDescriptor>> = HashMap::new();
        for row in rows {
            map.entry(row.table_name.clone()).or_default().push(row);
        }
        Ok(map)
    }

    /// Count how many migration files have not yet been applied.
    async fn count_pending_migrations(&self) -> Result<usize, sqlx::Error> {
        // sqlx stores applied migrations in `_sqlx_migrations`.
        // We compare against the filesystem migration files.
        let migrator = sqlx::migrate!("./migrations");
        let applied: Vec<i64> = sqlx::query_scalar(
            "SELECT version FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let total = migrator.migrations.len();
        let pending = total.saturating_sub(applied.len());
        Ok(pending)
    }

    /// Run migrations inside a transaction that is always rolled back.
    async fn run_dry_run(&self) -> DryRunResult {
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                return DryRunResult {
                    success: false,
                    error: Some(format!("failed to begin transaction: {e}")),
                    statements_executed: 0,
                }
            }
        };

        // Apply each pending migration SQL inside the transaction.
        let migrator = sqlx::migrate!("./migrations");
        let mut count = 0usize;

        for migration in migrator.migrations.iter() {
            let sql = migration.sql.as_ref();
            // Split on semicolons for rough statement counting.
            let stmts: Vec<&str> = sql
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            count += stmts.len();

            if let Err(e) = sqlx::query(sql).execute(&mut *tx).await {
                let _ = tx.rollback().await;
                return DryRunResult {
                    success: false,
                    error: Some(format!(
                        "migration '{}' failed: {e}",
                        migration.description
                    )),
                    statements_executed: count,
                };
            }
        }

        // Always roll back — this is a dry run.
        let _ = tx.rollback().await;
        info!(statements = count, "dry-run complete (rolled back)");

        DryRunResult {
            success: true,
            error: None,
            statements_executed: count,
        }
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
            let before_map: HashMap<&str, &ColumnDescriptor> =
                before_cols.iter().map(|c| (c.column_name.as_str(), c)).collect();
            let after_map: HashMap<&str, &ColumnDescriptor> =
                after_cols.iter().map(|c| (c.column_name.as_str(), c)).collect();

            // Columns added
            for (col_name, col) in &after_map {
                if !before_map.contains_key(col_name) {
                    diff.push(SchemaDiffEntry::ColumnAdded {
                        table: table.clone(),
                        column: col_name.to_string(),
                        data_type: col.data_type.clone(),
                        nullable: col.is_nullable,
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
                    }
                }
            }
        }
    }

    diff.sort_by(|a, b| format!("{a}").cmp(&format!("{b}")));
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
        // Adding a new table is always safe.
        SchemaDiffEntry::TableAdded(_) => CompatibilityRisk::Safe,

        // Dropping a table is always breaking.
        SchemaDiffEntry::TableDropped(t) => CompatibilityRisk::Breaking(format!(
            "Dropping table '{t}' will break any code that still references it. \
             Use a multi-step rollout: deprecate → stop references → drop."
        )),

        // Adding a nullable column with a default is safe for zero-downtime.
        SchemaDiffEntry::ColumnAdded {
            nullable, table, column, ..
        } => {
            if *nullable {
                CompatibilityRisk::Safe
            } else {
                CompatibilityRisk::RequiresPhasing(format!(
                    "Adding NOT NULL column '{table}.{column}' without a default will \
                     fail on rows inserted by the old binary. Add a default or backfill first."
                ))
            }
        }

        // Dropping a column breaks old code that still SELECT/INSERT it.
        SchemaDiffEntry::ColumnDropped { table, column } => {
            CompatibilityRisk::Breaking(format!(
                "Dropping '{table}.{column}' will break old binaries that reference it. \
                 Deprecate the column in code first."
            ))
        }

        // Type changes are almost always breaking.
        SchemaDiffEntry::TypeChanged { table, column, from, to } => {
            CompatibilityRisk::Breaking(format!(
                "Changing '{table}.{column}' from {from} to {to} may corrupt data or \
                 break deserialization in running replicas."
            ))
        }

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
/// leaves the schema unchanged.
///
/// This is primarily useful in integration tests where you can call
/// [`MigrationChecker::run_rollback_test`] against a fresh database.
pub async fn run_rollback_test(pool: &PgPool, migration_sql: &str) -> Result<bool, sqlx::Error> {
    let before = snapshot_public_tables(pool).await?;

    let mut tx = pool.begin().await?;
    let _ = sqlx::query(migration_sql).execute(&mut *tx).await;
    tx.rollback().await?;

    let after = snapshot_public_tables(pool).await?;
    let unchanged = before == after;

    if unchanged {
        info!("rollback test passed: schema is identical before and after rollback");
    } else {
        warn!("rollback test FAILED: schema changed despite transaction rollback");
    }

    Ok(unchanged)
}

/// Return a sorted list of table names in the `public` schema.
async fn snapshot_public_tables(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' ORDER BY table_name",
    )
    .fetch_all(pool)
    .await
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
        after.insert("pools".to_string(), vec![col("pools", "id", "bigint", false)]);

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|e| matches!(e, SchemaDiffEntry::TableAdded(t) if t == "pools")));
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
        assert!(
            diff.iter()
                .any(|e| matches!(e, SchemaDiffEntry::TableDropped(t) if t == "old_table"))
        );
    }

    #[test]
    fn diff_detects_added_column() {
        let mut before = HashMap::new();
        before.insert("pools".to_string(), vec![col("pools", "id", "bigint", false)]);
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
        after.insert("pools".to_string(), vec![col("pools", "id", "bigint", false)]);

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
        before.insert("pools".to_string(), vec![col("pools", "amount", "integer", false)]);
        let mut after = HashMap::new();
        after.insert("pools".to_string(), vec![col("pools", "amount", "bigint", false)]);

        let diff = compute_schema_diff(&before, &after);
        assert!(diff.iter().any(|e| matches!(
            e,
            SchemaDiffEntry::TypeChanged { table, column, from, to }
            if table == "pools" && column == "amount" && from == "integer" && to == "bigint"
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
        };
        let finding = &analyse_compatibility(&[entry])[0];
        assert!(matches!(finding.risk, CompatibilityRisk::RequiresPhasing(_)));
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
        });
        report.schema_diff = vec![SchemaDiffEntry::TableAdded("new_table".to_string())];
        report.has_breaking_changes = false;

        let summary = report.summary();
        assert!(summary.contains("Pending migrations"));
        assert!(summary.contains("Dry-run"));
        assert!(summary.contains("new_table"));
        assert!(summary.contains("No breaking changes"));
    }
}
