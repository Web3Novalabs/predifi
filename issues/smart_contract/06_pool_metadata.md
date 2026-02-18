# Issue 305: Add Description and Metadata URL to Pool

## Description

Expand the `Pool` struct to include a short description string and a URL (e.g., IPFS link) for extended metadata. This allows the frontend to display more context about the event being predicted.

## Tasks

- Update `Pool` struct definition in `lib.rs`.
- Modify `create_pool` function to accept these new parameters.
- Update events to include metadata indicators.

## Dependencies

- Issue #300
- Issue #302
