-- Migration: Initial tables for predifi

CREATE TABLE market_category (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category_id INTEGER REFERENCES market_category(id),
    image_url TEXT,
    event_source_url TEXT,
    start_time BIGINT,
    lock_time BIGINT,
    end_time BIGINT,
    option1 TEXT,
    option2 TEXT,
    min_bet_amount NUMERIC,
    max_bet_amount NUMERIC,
    creator_fee SMALLINT,
    is_private BOOLEAN,
    creator_address TEXT,
    created_timestamp BIGINT,
    status TEXT DEFAULT 'active'
);

CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE market_tags (
    id SERIAL PRIMARY KEY,
    market_id INTEGER REFERENCES market(id) ON DELETE CASCADE,
    tag_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    UNIQUE(market_id, tag_id)
);

CREATE TABLE pool (
    id SERIAL PRIMARY KEY,
    market_id INTEGER REFERENCES market(id),
    name TEXT NOT NULL,
    type SMALLINT NOT NULL,
    description TEXT,
    image_url TEXT,
    event_source_url TEXT,
    start_time BIGINT,
    lock_time BIGINT,
    end_time BIGINT,
    option1 TEXT,
    option2 TEXT,
    min_bet_amount NUMERIC,
    max_bet_amount NUMERIC,
    creator_fee SMALLINT,
    is_private BOOLEAN,
    category_id INTEGER REFERENCES market_category(id)
);

-- Add pool_status enum type
DO $$ BEGIN
    CREATE TYPE pool_status AS ENUM ('Active', 'Locked', 'Settled', 'Closed');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

ALTER TABLE pool ADD COLUMN status pool_status NOT NULL DEFAULT 'Active';
CREATE INDEX idx_pool_status ON pool(status);

CREATE TABLE user_pool (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    pool_id INTEGER REFERENCES pool(id),
    amount_staked NUMERIC NOT NULL
);
