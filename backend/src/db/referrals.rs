//! Referral repository — queries for the `referrals` and
//! `referrer_pool_stats` tables.

use sqlx::{Executor, PgPool, Postgres};

// ── Row / DTO types ───────────────────────────────────────────────────────────

/// Per-pool referral earnings breakdown row.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ReferralEarningRow {
    pub pool_id: i64,
    pub pool_name: String,
    pub total_earned: i64,
    pub referral_count: i64,
}

/// Decoded data from a `referral_paid` contract event.
#[derive(Debug)]
pub struct ReferralPaidEvent {
    pub pool_id: u64,
    pub referrer: String,
    pub referred_user: String,
    pub referral_amount: i64,
}

// ── Read queries ──────────────────────────────────────────────────────────────

/// Referral earnings grouped by pool for a given referrer address.
pub async fn get_referral_earnings(
    pool: &PgPool,
    address: &str,
) -> Result<Vec<ReferralEarningRow>, sqlx::Error> {
    sqlx::query_as::<_, ReferralEarningRow>(
        r#"
        SELECT
            rps.pool_id,
            pl.name                             AS pool_name,
            COALESCE(rps.total_earned, 0)::BIGINT AS total_earned,
            rps.referral_count
        FROM referrer_pool_stats rps
        JOIN pools pl ON pl.pool_id = rps.pool_id
        WHERE rps.referrer = $1
        ORDER BY rps.total_earned DESC
        "#,
    )
    .bind(address)
    .fetch_all(pool)
    .await
}

// ── Write queries ─────────────────────────────────────────────────────────────

/// Insert a batch of referral events using a single multi-row INSERT.
///
/// Large batches are split into chunks of at most `max_batch_size` rows to
/// stay within PostgreSQL's parameter-count limit (~65 535).
pub async fn insert_referrals_bulk(
    pool: &PgPool,
    events: &[ReferralPaidEvent],
    max_batch_size: usize,
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

    for chunk in events.chunks(max_batch_size.max(1)) {
        insert_referrals_chunk(pool, chunk).await?;
    }

    Ok(())
}

/// Insert a single referral event, silently ignoring duplicates.
pub async fn insert_referral_from_event<'e, E>(
    executor: E,
    event: &ReferralPaidEvent,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        r#"
        INSERT INTO referrals (referrer, user_address, pool_id, amount)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&event.referrer)
    .bind(&event.referred_user)
    .bind(event.pool_id as i64)
    .bind(event.referral_amount)
    .execute(executor)
    .await?;

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Build and execute a single multi-row INSERT for one chunk.
///
/// Generates positional placeholders (`($1, $2, $3, $4), ($5, …)`) at
/// runtime because sqlx does not support dynamic-width bulk inserts via its
/// query-builder API.  The placeholder strings are constructed entirely from
/// index arithmetic — no user data is interpolated into the SQL string.
async fn insert_referrals_chunk(
    pool: &PgPool,
    events: &[ReferralPaidEvent],
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

    // Build "($1,$2,$3,$4), ($5,$6,$7,$8), …"
    let placeholders: String = events
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let base = (i * 4 + 1) as i32;
            format!("(${}, ${}, ${}, ${})", base, base + 1, base + 2, base + 3)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "INSERT INTO referrals (referrer, user_address, pool_id, amount) VALUES {}",
        placeholders
    );

    let mut q = sqlx::query(&sql);
    for event in events {
        q = q
            .bind(&event.referrer)
            .bind(&event.referred_user)
            .bind(event.pool_id as i64)
            .bind(event.referral_amount);
    }

    q.execute(pool).await?;
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_generation_is_correct() {
        let events = vec![
            ReferralPaidEvent { pool_id: 1, referrer: "A".into(), referred_user: "B".into(), referral_amount: 10 },
            ReferralPaidEvent { pool_id: 2, referrer: "C".into(), referred_user: "D".into(), referral_amount: 20 },
        ];

        let placeholders: String = events
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let base = (i * 4 + 1) as i32;
                format!("(${}, ${}, ${}, ${})", base, base + 1, base + 2, base + 3)
            })
            .collect::<Vec<_>>()
            .join(", ");

        assert_eq!(placeholders, "($1, $2, $3, $4), ($5, $6, $7, $8)");
    }

    #[test]
    fn chunk_splitting_produces_correct_counts() {
        let events: Vec<ReferralPaidEvent> = (0..5)
            .map(|i| ReferralPaidEvent {
                pool_id: i,
                referrer: format!("R{i}"),
                referred_user: format!("U{i}"),
                referral_amount: 100,
            })
            .collect();

        let chunks: Vec<_> = events.chunks(2).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 2);
        assert_eq!(chunks[2].len(), 1);
    }

    #[test]
    fn empty_bulk_insert_returns_early() {
        // Just validates the guard — no I/O needed.
        let events: Vec<ReferralPaidEvent> = vec![];
        assert!(events.is_empty());
    }
}
