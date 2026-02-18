# Issue 300: Standardize Workspace and Crate Structure

## Description

Reorganize the `contract/` directory to follow a standard multi-crate Soroban workspace. Move `predifi-contract` and `access-control` into a clean structure and ensure the root `Cargo.toml` correctly manages these as members.

## Tasks

- Clean up `contract/contracts/` directory layout.
- Update root `Cargo.toml` with consistent workspace members.
- Ensure all crates share common dependencies via workspace configuration.

## Dependencies

- None
