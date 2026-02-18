# Issue 324: Implement Efficient Search/Filter Helper Functions

## Description

While most indexing happens off-chain, providing basic on-chain filtering (e.g., "get latest 10 pools in Category X") can be useful for light clients and basic contract interaction.

## Tasks

- Maintain a secondary index of pool IDs by category symbol.
- Implement `get_pools_by_category` with pagination.
- Ensure indexes are updated correctly during pool creation.

## Dependencies

- Issue #306
- Issue #302
