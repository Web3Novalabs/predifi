-- Add migration script here
CREATE TABLE IF NOT EXISTS validators (
    contract_address TEXT PRIMARY KEY,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_validators_contract_address ON validators(contract_address);