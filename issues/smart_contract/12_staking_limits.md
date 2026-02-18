# Issue 311: Add Minimum and Maximum Stake Limits per User

## Description

To protect users and manage risk, implement per-user staking limits for each market. This includes a global minimum stake (already partially present) and a newly introduced maximum stake limit per prediction.

## Tasks

- Add `max_stake` field to the `Pool` struct.
- Update `place_prediction` to enforce both `min_stake` and `max_stake`.
- Allow the contract owner to update these limits for existing pools if necessary.

## Dependencies

- Issue #305
- Issue #307
