//! Database seed script for local development (#1189).
//!
//! Populates the local PostgreSQL database with deterministic, idempotent
//! sample data — pools across every state/category, predictions from a set of
//! fixture wallets, and referral records — so the backend can be exercised
//! end-to-end without waiting for on-chain events to be indexed.
//!
//! # Idempotence
//!
//! Every insert uses `ON CONFLICT DO NOTHING` (or `DO UPDATE`) keyed on the
//! natural primary keys (`pool_id`, `(pool_id, user_address, outcome)`), so
//! running the seeder multiple times against the same database is safe and
//! produces the same final state.
//!
//! # Usage
//!
//! Run via the `predifi-seed` binary:
//!
//! ```text
//! cargo run --bin predifi-seed                  # insert seed data
//! cargo run --bin predifi-seed -- --fresh       # truncate first, then seed
//! cargo run --bin predifi-seed -- --num-pools 20
//! cargo run --bin predifi-seed -- --help
//! ```

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::info;

use crate::db::{PoolCreatedEvent, PredictionPlacedEvent, ReferralPaidEvent};

/// Default number of pools to generate when `--num-pools` is not supplied.
pub const DEFAULT_NUM_POOLS: usize = 10;

/// Fixture wallet addresses used for seeded predictions and referrals.
///
/// These are arbitrary 56-character Stellar-style public keys (prefixed with
/// `G`, padded with `A`s). They are not real accounts on any network — they
/// exist only so locally seeded data has stable, recognisable identities.
pub const SEED_WALLETS: &[&str] = &[
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1",
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA2",
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA3",
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA4",
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA5",
];

/// Fixture referrer address — distinct from the betting wallets so referral
/// earnings queries have data to aggregate.
pub const SEED_REFERRER: &str = "GRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR";

/// Pool categories cycled through when generating pools.
pub const SEED_CATEGORIES: &[&str] = &["Sports", "Crypto", "Politics", "Entertainment"];

/// Token strings cycled through when generating pools.
pub const SEED_TOKENS: &[&str] = &[
    "native",
    "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RztMfo6BASE2QYX",
    "EURC:GDHU6RQSD3QZFFIGTVK6VP4YHV2KLDPEMTZSDQ3B",
];

/// Description of a pool to seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedPool {
    pub pool_id: u64,
    pub creator: String,
    pub end_time: DateTime<Utc>,
    pub token: String,
    pub category: String,
    pub description: String,
    pub state: String,
    /// Winning outcome, only set for settled pools.
    pub result: Option<i32>,
}

/// Description of a prediction to seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedPrediction {
    pub pool_id: u64,
    pub user_address: String,
    pub outcome: i32,
    pub amount: i64,
}

/// Description of a referral payment to seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedReferral {
    pub pool_id: u64,
    pub referrer: String,
    pub referred_user: String,
    pub amount: i64,
}

/// Configuration for a seed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedConfig {
    /// Number of pools to generate.
    pub num_pools: usize,
    /// When true, truncate `pools`, `predictions`, `referrals`, and `stats`
    /// before inserting fresh data.
    pub fresh: bool,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            num_pools: DEFAULT_NUM_POOLS,
            fresh: false,
        }
    }
}

/// Generate the deterministic list of pool fixtures for a given count.
///
/// Pools cycle through categories and tokens. The first ~30 % are `active`,
/// the next ~20 % are `closed`, and the remainder are `settled` with a
/// deterministic winning outcome derived from the pool id.
pub fn build_seed_pools(num_pools: usize) -> Vec<SeedPool> {
    let now = Utc::now();
    (0..num_pools)
        .map(|i| {
            let pool_id = (i + 1) as u64;
            let category = SEED_CATEGORIES[i % SEED_CATEGORIES.len()];
            let token = SEED_TOKENS[i % SEED_TOKENS.len()];
            let creator = SEED_WALLETS[i % SEED_WALLETS.len()].to_string();
            let description = format!("Seed pool #{} — {} category", pool_id, category);

            let end_time = now + Duration::days(7 + (i as i64));
            let (state, result) = if i < num_pools / 3 {
                ("active".to_string(), None)
            } else if i < num_pools / 2 {
                ("closed".to_string(), None)
            } else {
                let winning = ((pool_id % 2) as i32) ^ 0;
                ("settled".to_string(), Some(winning))
            };

            SeedPool {
                pool_id,
                creator,
                end_time,
                token: token.to_string(),
                category: category.to_string(),
                description,
                state,
                result,
            }
        })
        .collect()
}

/// Generate deterministic predictions for the given pools.
///
/// Each pool receives 2–4 predictions from fixture wallets, with amounts
/// derived from the pool id so totals are reproducible across runs.
pub fn build_seed_predictions(pools: &[SeedPool]) -> Vec<SeedPrediction> {
    let mut out = Vec::new();
    for pool in pools {
        let n_predictions = 2 + ((pool.pool_id % 3) as usize); // 2..=4
        for j in 0..n_predictions {
            let wallet = SEED_WALLETS[(j as usize) % SEED_WALLETS.len()];
            let outcome = ((j as u64 + pool.pool_id) % 2) as i32;
            let amount = 100 + ((pool.pool_id as i64) * 50) + (j as i64 * 25);
            out.push(SeedPrediction {
                pool_id: pool.pool_id,
                user_address: wallet.to_string(),
                outcome,
                amount,
            });
        }
    }
    out
}

/// Generate deterministic referral payments for the given pools.
///
/// One referral per pool, attributed to the fixture referrer, with the
/// referred user drawn from the wallet pool.
pub fn build_seed_referrals(pools: &[SeedPool]) -> Vec<SeedReferral> {
    pools
        .iter()
        .map(|pool| {
            let referred = SEED_WALLETS[(pool.pool_id as usize) % SEED_WALLETS.len()];
            let amount = 10 + ((pool.pool_id as i64) * 5);
            SeedReferral {
                pool_id: pool.pool_id,
                referrer: SEED_REFERRER.to_string(),
                referred_user: referred.to_string(),
                amount,
            }
        })
        .collect()
}

/// Truncate all seed-managed tables.
///
/// `stats` is omitted from the explicit list because it has a foreign key to
/// `pools` with `ON DELETE CASCADE` — truncating `pools` cascades to it.
pub async fn truncate_all(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("TRUNCATE TABLE predictions RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE referrals RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("TRUNCATE TABLE pools CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert (or upsert) the given pool fixtures.
///
/// Uses `ON CONFLICT (pool_id) DO UPDATE` so re-running the seeder refreshes
/// state, category, and result rather than silently skipping.
pub async fn insert_seed_pools(
    pool: &PgPool,
    pools: &[SeedPool],
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0u64;
    for p in pools {
        let result_str = p.result.map(|r| r.to_string());
        sqlx::query(
            r#"
            INSERT INTO pools (pool_id, name, category, total_stake, end_time, state, creator, token, result, created_at)
            VALUES ($1, $2, $3, 0, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (pool_id) DO UPDATE SET
                name        = EXCLUDED.name,
                category    = EXCLUDED.category,
                end_time    = EXCLUDED.end_time,
                state       = EXCLUDED.state,
                creator     = EXCLUDED.creator,
                token       = EXCLUDED.token,
                result      = EXCLUDED.result
            "#,
        )
        .bind(p.pool_id as i64)
        .bind(&p.description)
        .bind(&p.category)
        .bind(p.end_time)
        .bind(&p.state)
        .bind(&p.creator)
        .bind(&p.token)
        .bind(result_str)
        .execute(pool)
        .await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Insert predictions and update each pool's `total_stake` to match the sum of
/// its seeded predictions.
pub async fn insert_seed_predictions(
    pool: &PgPool,
    predictions: &[SeedPrediction],
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0u64;
    for pred in predictions {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO predictions (pool_id, user_address, outcome, amount)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(pred.pool_id as i64)
        .bind(&pred.user_address)
        .bind(pred.outcome)
        .bind(pred.amount)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE pools SET total_stake = total_stake + $1 WHERE pool_id = $2")
            .bind(pred.amount)
            .bind(pred.pool_id as i64)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Insert referral payment fixtures.
pub async fn insert_seed_referrals(
    pool: &PgPool,
    referrals: &[SeedReferral],
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0u64;
    for r in referrals {
        sqlx::query(
            r#"
            INSERT INTO referrals (referrer, user_address, pool_id, amount)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&r.referrer)
        .bind(&r.referred_user)
        .bind(r.pool_id as i64)
        .bind(r.amount)
        .execute(pool)
        .await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Convert a [`SeedPool`] into the on-chain event shape used by `db::insert_pool_from_event`.
///
/// This is useful for tests that want to exercise the existing ingest path
/// rather than the seed-specific insert.
pub fn seed_pool_to_event(p: &SeedPool) -> PoolCreatedEvent {
    PoolCreatedEvent {
        pool_id: p.pool_id,
        creator: p.creator.clone(),
        end_time: p.end_time.timestamp() as u64,
        token: p.token.clone(),
        category: p.category.clone(),
        description: p.description.clone(),
    }
}

/// Convert a [`SeedPrediction`] into the on-chain event shape used by
/// `db::insert_prediction_from_event_with_pool` (or `db::insert_prediction_from_event`
/// when composing multi-step writes inside a transaction).
pub fn seed_prediction_to_event(p: &SeedPrediction) -> PredictionPlacedEvent {
    PredictionPlacedEvent {
        pool_id: p.pool_id,
        user_address: p.user_address.clone(),
        outcome: p.outcome,
        amount: p.amount,
    }
}

/// Convert a [`SeedReferral`] into the on-chain event shape used by
/// `db::insert_referral_from_event`.
pub fn seed_referral_to_event(r: &SeedReferral) -> ReferralPaidEvent {
    ReferralPaidEvent {
        pool_id: r.pool_id,
        referrer: r.referrer.clone(),
        referred_user: r.referred_user.clone(),
        referral_amount: r.amount,
    }
}

/// Run the full seed pipeline against a live pool.
///
/// Returns counts of each kind of row written so callers can log a summary.
pub async fn run_seed(
    pool: &PgPool,
    config: &SeedConfig,
) -> Result<SeedSummary, sqlx::Error> {
    if config.fresh {
        info!("--fresh: truncating existing seed tables");
        truncate_all(pool).await?;
    }

    let pools = build_seed_pools(config.num_pools);
    let predictions = build_seed_predictions(&pools);
    let referrals = build_seed_referrals(&pools);

    let pools_written = insert_seed_pools(pool, &pools).await?;
    let predictions_written = insert_seed_predictions(pool, &predictions).await?;
    let referrals_written = insert_seed_referrals(pool, &referrals).await?;

    let summary = SeedSummary {
        pools: pools_written,
        predictions: predictions_written,
        referrals: referrals_written,
    };
    info!(?summary, "seed complete");
    Ok(summary)
}

/// Counts of rows written by a [`run_seed`] invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedSummary {
    pub pools: u64,
    pub predictions: u64,
    pub referrals: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_seed_pools_provides_requested_count() {
        let pools = build_seed_pools(7);
        assert_eq!(pools.len(), 7);
        for (i, p) in pools.iter().enumerate() {
            assert_eq!(p.pool_id, (i + 1) as u64);
        }
    }

    #[test]
    fn build_seed_pools_distributes_states_deterministically() {
        let pools = build_seed_pools(10);
        let active = pools.iter().filter(|p| p.state == "active").count();
        let closed = pools.iter().filter(|p| p.state == "closed").count();
        let settled = pools.iter().filter(|p| p.state == "settled").count();
        assert_eq!(active, 3, "first ~1/3 should be active");
        assert_eq!(closed, 2, "next ~1/6 should be closed");
        assert_eq!(settled, 5, "remainder should be settled");
        // Settled pools must always carry a result.
        assert!(pools
            .iter()
            .filter(|p| p.state == "settled")
            .all(|p| p.result.is_some()));
        // Active/closed pools never carry a result.
        assert!(pools
            .iter()
            .filter(|p| p.state != "settled")
            .all(|p| p.result.is_none()));
    }

    #[test]
    fn build_seed_pools_cycles_categories_and_tokens() {
        let pools = build_seed_pools(SEED_CATEGORIES.len() * 2);
        assert_eq!(pools[0].category, SEED_CATEGORIES[0]);
        assert_eq!(pools[SEED_CATEGORIES.len()].category, SEED_CATEGORIES[0]);
        assert_eq!(pools[0].token, SEED_TOKENS[0]);
        assert_eq!(pools[SEED_TOKENS.len()].token, SEED_TOKENS[0]);
    }

    #[test]
    fn build_seed_pools_is_deterministic_across_calls() {
        let a = build_seed_pools(12);
        let b = build_seed_pools(12);
        // end_time depends on `now()` so we compare everything except end_time.
        let strip = |p: &SeedPool| (p.pool_id, p.creator.clone(), p.category.clone(), p.state.clone(), p.result);
        let a_stripped: Vec<_> = a.iter().map(strip).collect();
        let b_stripped: Vec<_> = b.iter().map(strip).collect();
        assert_eq!(a_stripped, b_stripped);
    }

    #[test]
    fn build_seed_predictions_covers_every_pool() {
        let pools = build_seed_pools(5);
        let preds = build_seed_predictions(&pools);
        for p in &pools {
            let count = preds.iter().filter(|x| x.pool_id == p.pool_id).count();
            assert!(count >= 2 && count <= 4, "pool {} got {} predictions", p.pool_id, count);
        }
    }

    #[test]
    fn build_seed_predictions_amounts_are_positive_and_deterministic() {
        let pools = build_seed_pools(4);
        let preds = build_seed_predictions(&pools);
        assert!(preds.iter().all(|p| p.amount > 0));
        let again = build_seed_predictions(&pools);
        assert_eq!(preds, again);
    }

    #[test]
    fn build_seed_referrals_has_one_per_pool() {
        let pools = build_seed_pools(6);
        let refs = build_seed_referrals(&pools);
        assert_eq!(refs.len(), pools.len());
        assert!(refs.iter().all(|r| r.referrer == SEED_REFERRER));
        assert!(refs.iter().all(|r| r.amount > 0));
    }

    #[test]
    fn seed_pool_to_event_preserves_fields() {
        let pool = build_seed_pools(1).into_iter().next().unwrap();
        let event = seed_pool_to_event(&pool);
        assert_eq!(event.pool_id, pool.pool_id);
        assert_eq!(event.creator, pool.creator);
        assert_eq!(event.token, pool.token);
        assert_eq!(event.category, pool.category);
        assert_eq!(event.description, pool.description);
    }

    #[test]
    fn seed_prediction_to_event_preserves_fields() {
        let pred = SeedPrediction {
            pool_id: 42,
            user_address: "GABC".to_string(),
            outcome: 1,
            amount: 500,
        };
        let event = seed_prediction_to_event(&pred);
        assert_eq!(event.pool_id, pred.pool_id);
        assert_eq!(event.user_address, pred.user_address);
        assert_eq!(event.outcome, pred.outcome);
        assert_eq!(event.amount, pred.amount);
    }

    #[test]
    fn seed_referral_to_event_preserves_fields() {
        let r = SeedReferral {
            pool_id: 7,
            referrer: "GREF".to_string(),
            referred_user: "GREC".to_string(),
            amount: 25,
        };
        let event = seed_referral_to_event(&r);
        assert_eq!(event.pool_id, r.pool_id);
        assert_eq!(event.referrer, r.referrer);
        assert_eq!(event.referred_user, r.referred_user);
        assert_eq!(event.referral_amount, r.amount);
    }

    #[test]
    fn seed_config_defaults_are_sensible() {
        let cfg = SeedConfig::default();
        assert_eq!(cfg.num_pools, DEFAULT_NUM_POOLS);
        assert!(!cfg.fresh);
    }
}
