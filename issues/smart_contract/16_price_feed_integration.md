# Issue 315: Add Support for External Price Feeds (e.g., Pyth or Band)

## Description

Integrate with established Soroban oracle solutions like Pyth or Band to automate the resolution of price-based prediction markets (e.g., "Will ETH be above $3k at time X?").

## Tasks

- Research and select a primary price feed provider.
- Implement a bridge/adapter within the contract to query these feeds.
- Automate market resolution based on the feed's timestamped data.

## Dependencies

- Issue #313
