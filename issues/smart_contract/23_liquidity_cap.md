# Issue 322: Implement Max Cap on Total Pool Liquidity

## Description

Provide pool creators with the option to set a maximum total stake cap for a market. Once the cap is reached, no more predictions can be placed. This helps in managing risk and ensuring liquidity remains balanced.

## Tasks

- Add `max_total_stake` field to `Pool` struct.
- Update `place_prediction` to enforce the global stake cap.
- Logic to allow increasing the cap by the creator before market end.

## Dependencies

- Issue #305
- Issue #311
