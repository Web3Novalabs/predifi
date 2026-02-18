# Issue 342: Automated Protocol Revenue Splitter

## Description

Instead of a single Treasury address, implement a splitter contract or logic that automatically distributes collected fees between protocol development, insurance pools, and potential liquidity providers.

## Tasks

- Implement a configurable split ratio (percentage-based).
- Support multiple destination addresses for the split.
- Ensure overflow protection during large distribution calculations.

## Dependencies

- Issue #304
- Issue #319
- Issue #317
