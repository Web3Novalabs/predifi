# Backup and Disaster Recovery Plan

Covers automated database backups, point-in-time recovery (PITR), cross-region
replication, RTO/RPO targets, and the quarterly recovery drill for PrediFi.

## RTO / RPO targets

| Tier | Scenario | RTO | RPO |
| ---- | -------- | --- | --- |
| Tier 1 | Primary Postgres failure / data loss | ≤ 1 hour | ≤ 5 minutes |
| Tier 2 | Availability-zone or region loss | ≤ 4 hours | ≤ 15 minutes |
| Tier 3 | Accidental deletion or bad migration (logical restore) | ≤ 24 hours | ≤ 24 hours |

- **RTO** — time from incident declaration to the service serving traffic again.
- **RPO** — maximum acceptable data loss, measured as the age of the newest
  recoverable backup.

## Automated backups (primary region)

Configured in `terraform/modules/postgres/main.tf`:

- `backup_retention_period = 30` — 30 days of automated backups; this also
  enables RDS **point-in-time recovery**, which can restore to any second
  inside the retention window.
- `backup_window = 03:00-04:00` (UTC), outside peak traffic.
- `copy_tags_to_snapshot = true` — snapshots inherit ownership/rotation tags.
- `delete_automated_backups = false` — automated backups survive instance
  deletion.
- `skip_final_snapshot = false` + `final_snapshot_identifier` — a final
  encrypted snapshot is always taken on destroy.
- Storage is encrypted and `deletion_protection = true`.

Backups are only as good as their restore path: every backup policy change
must also update the restore runbook below and be exercised by the quarterly
drill.

## Point-in-time recovery (PITR)

1. Identify the target time (UTC) — the last known-good moment before the
   incident.
2. Create the restore:
   ```bash
   aws rds restore-db-instance-to-point-in-time \
     --source-db-instance-identifier predifi-pg \
     --target-db-instance-identifier predifi-pg-restore \
     --use-latest-restorable-time \
     --restore-type copy-on-write
   ```
   For an exact timestamp, replace `--use-latest-restorable-time` with
   `--restore-to-time <ISO-8601>`.
3. Point the backend at the restored instance (`PREDIFI_DATABASE_URL`),
   deploy, and verify with the health endpoints (`/healthz`, `/ready`).
4. Once verified, promote the restored instance to primary (rename / DNS /
   endpoint swap) and take a fresh snapshot before resuming writes.

RPO for PITR is bounded by transaction-log shipping (seconds); the documented
RPO of ≤ 5 minutes is the operational target.

## Cross-region replication

- **Manual snapshots** are copied to a secondary region with a 7-day expiry:
  ```bash
  aws rds copy-db-snapshot \
    --source-db-snapshot-identifier predifi-pg-final \
    --target-db-snapshot-identifier predifi-pg-final-dr \
    --copy-tags \
    --region <secondary-region>
  ```
- **Automated cross-region copies** are handled by an AWS Backup plan in the
  `terraform/modules/postgres` module: a daily copy job writes to a backup
  vault in the secondary region with a 35-day retention window, satisfying the
  Tier 2 RPO.
- Drift/encryption keys and `PREDIFI_SECRET_KEY` are stored in the secondary
  region's secrets manager so a region failover can decrypt restores.

## Region failover runbook (Tier 2)

1. Declare the incident and announce the recovery lead.
2. Confirm the secondary region's vault has a restorable copy (< 15 min old).
3. Restore into the secondary region (PITR or snapshot restore).
4. Recreate required infra: `terraform -chdir=terraform apply` for the DR
   environment (with `-var` overrides for the secondary region).
5. Update DNS / load-balancer target to the restored endpoint.
6. Verify health endpoints and a smoke transaction (read + one test write).
7. Communicate recovery; capture the timeline for the post-mortem.

## Quarterly recovery test

Scheduled each calendar quarter (tracked in the issue tracker):

1. Provision an isolated scratch instance from the latest backup — never from
   production.
2. Restore to a fixed point-in-time and measure the elapsed restore time.
3. Assert the measured **RTO** and **RPO** match the targets above; record
   actuals in the drill report.
4. Run `backend/tests` smoke queries against the restored database to confirm
   schema and seed integrity.
5. File a report (or post-mortem if a target was missed) and file follow-up
   issues for any gap found.

A drill that has not been run within a quarter is itself an incident — raise
it with the on-call lead.
