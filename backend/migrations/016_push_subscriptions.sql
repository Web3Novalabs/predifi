-- Migration: push notification subscriptions (#1929 follow-up)
--
-- Stores device / browser push tokens per wallet so the notification sweep can
-- deliver alerts over the `push` channel (already allowed by
-- `notification_delivery_log.channel`). Delivery goes through a configurable
-- push gateway (`PREDIFI_PUSH_GATEWAY_URL`); tokens the gateway reports as
-- gone are pruned automatically.

CREATE TABLE IF NOT EXISTS push_subscriptions (
    id           BIGSERIAL    PRIMARY KEY,
    user_address TEXT         NOT NULL,
    token        TEXT         NOT NULL,
    platform     VARCHAR(16)  NOT NULL DEFAULT 'web'
                              CHECK (platform IN ('web', 'ios', 'android')),
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_address, token)
);

CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user
    ON push_subscriptions (user_address);
