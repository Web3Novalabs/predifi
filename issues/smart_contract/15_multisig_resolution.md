# Issue 314: Implement Multi-Signature Resolution Logic

## Description

For high-value or contentious markets, require multiple authorized resolutions (e.g., 2-of-3 oracles) before a pool is finalized. This adds a layer of decentralization and security to the resolution process.

## Tasks

- Update `Pool` state to track multiple resolution votes.
- Implement logic to aggregate votes and trigger final resolution when a threshold is met.
- Add handling for resolution conflicts/disagreements.

## Dependencies

- Issue #313
- Issue #302
