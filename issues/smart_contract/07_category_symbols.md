# Issue 306: Implement Market Categories as Symbols

## Description

Instead of generic strings, use Soroban `Symbol` types for market categories (e.g., `Sports`, `Finance`, `Crypto`). This improves storage efficiency and allows for easier filtering in the contract and indexer.

## Tasks

- Define a list of standard categories as constants or symbols.
- Update `create_pool` to validate the provided category symbol.
- Update storage keys to include category-based indexing if possible.

## Dependencies

- Issue #305
