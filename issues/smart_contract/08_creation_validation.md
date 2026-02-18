# Issue 307: Strengthen Pool Creation Parameter Validation

## Description

The `create_pool` function needs stricter checks on parameters like `end_time` (minimum duration), `min_stake` (sanity checks), and total number of options. This prevents the creation of "corrupt" or impossible markets.

## Tasks

- Impose a minimum pool duration (e.g., 1 hour).
- Validate that `options_count` does not exceed a reasonable limit (e.g., 100).
- Add checks for null/zero addresses for tokens.

## Dependencies

- Issue #301
