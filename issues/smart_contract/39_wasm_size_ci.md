# Issue 338: Set up Automated WASM Size Checking in CI

## Description

Integrate a check in the GitHub Actions workflow to monitor the size of the compiled WASM file. This ensures the contract remains within Soroban's limits and alerts developers to regressions.

## Tasks

- Add a step to `.github/workflows/` to build and check WASM size.
- Set a threshold and cause build failure if exceeded.
- Log artifacts of the build sizes over time.

## Dependencies

- Issue #300
