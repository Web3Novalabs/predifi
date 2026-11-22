-- Migration: pre-launch waitlist sign-ups (#2101)
--
-- Backs `POST /api/v1/waitlist`. Emails are stored trimmed and lower-cased by
-- the handler; the CHECK keeps that invariant true for any other writer, so the
-- UNIQUE constraint really does de-duplicate case variants of the same address.

CREATE TABLE IF NOT EXISTS waitlist (
    id          BIGSERIAL    PRIMARY KEY,
    name        TEXT         NOT NULL,
    email       TEXT         NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT waitlist_email_key UNIQUE (email),
    CONSTRAINT waitlist_email_lowercase CHECK (email = lower(email))
);
