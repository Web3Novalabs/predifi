# Issue 316: Implement Initial Liquidity Provisioning for Markets

## Description

Allow pool creators to provide initial "house" liquidity to a market. This provides an immediate reward for early betters and increases market depth.

## Tasks

- Update `create_pool` to accept an optional initial liquidity amount.
- Ensure initial liquidity is transferred from the creator to the contract.
- Logic to treat initial liquidity as part of the total stake but handled separately for fee calculations.

## Dependencies

- Issue #305
- Issue #312
