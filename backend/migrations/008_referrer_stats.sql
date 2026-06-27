-- Migration: pre-aggregated referrer statistics for fast volume reads
--
-- Maintains incremental aggregates via trigger so referral endpoints avoid
-- full-table SUM/COUNT(DISTINCT) scans on every request.

CREATE TABLE IF NOT EXISTS referrer_users (
    referrer      TEXT NOT NULL,
    user_address  TEXT NOT NULL,
    PRIMARY KEY (referrer, user_address)
);

CREATE TABLE IF NOT EXISTS referrer_stats (
    referrer      TEXT            PRIMARY KEY,
    total_volume  NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    unique_users  BIGINT          NOT NULL DEFAULT 0,
    last_updated  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS referrer_pool_stats (
    referrer        TEXT            NOT NULL,
    pool_id         BIGINT          NOT NULL,
    total_earned    NUMERIC(32, 7)  NOT NULL DEFAULT 0,
    referral_count  BIGINT          NOT NULL DEFAULT 0,
    last_updated    TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (referrer, pool_id)
);

-- Backfill from existing referral rows.
INSERT INTO referrer_users (referrer, user_address)
SELECT DISTINCT referrer, user_address FROM referrals
ON CONFLICT DO NOTHING;

INSERT INTO referrer_stats (referrer, total_volume, unique_users, last_updated)
SELECT
    referrer,
    COALESCE(SUM(amount), 0),
    COUNT(DISTINCT user_address),
    NOW()
FROM referrals
GROUP BY referrer
ON CONFLICT (referrer) DO UPDATE SET
    total_volume = EXCLUDED.total_volume,
    unique_users = EXCLUDED.unique_users,
    last_updated = EXCLUDED.last_updated;

INSERT INTO referrer_pool_stats (referrer, pool_id, total_earned, referral_count, last_updated)
SELECT
    referrer,
    pool_id,
    COALESCE(SUM(amount), 0),
    COUNT(*),
    NOW()
FROM referrals
GROUP BY referrer, pool_id
ON CONFLICT (referrer, pool_id) DO UPDATE SET
    total_earned = EXCLUDED.total_earned,
    referral_count = EXCLUDED.referral_count,
    last_updated = EXCLUDED.last_updated;

CREATE OR REPLACE FUNCTION maintain_referrer_stats()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO referrer_users (referrer, user_address)
    VALUES (NEW.referrer, NEW.user_address)
    ON CONFLICT DO NOTHING;

    INSERT INTO referrer_stats (referrer, total_volume, unique_users, last_updated)
    VALUES (NEW.referrer, NEW.amount, 1, NOW())
    ON CONFLICT (referrer) DO UPDATE SET
        total_volume = referrer_stats.total_volume + NEW.amount,
        unique_users = (
            SELECT COUNT(*)::BIGINT FROM referrer_users WHERE referrer = NEW.referrer
        ),
        last_updated = NOW();

    INSERT INTO referrer_pool_stats (referrer, pool_id, total_earned, referral_count, last_updated)
    VALUES (NEW.referrer, NEW.pool_id, NEW.amount, 1, NOW())
    ON CONFLICT (referrer, pool_id) DO UPDATE SET
        total_earned = referrer_pool_stats.total_earned + NEW.amount,
        referral_count = referrer_pool_stats.referral_count + 1,
        last_updated = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_referrals_maintain_stats ON referrals;
CREATE TRIGGER trg_referrals_maintain_stats
    AFTER INSERT ON referrals
    FOR EACH ROW
    EXECUTE FUNCTION maintain_referrer_stats();
