# Issue 350: Advanced Trending Market Calculations

## Description

Implement on-chain logic or efficient event patterns to identify "Trending" markets based on rapid stake increases. This helps the frontend feature high-engagement markets.

## Tasks

- Add "last update" timestamps to volume storage.
- Implement a window-based volume tracker (e.g., volume in last 1 hour).
- Optimize for high-frequency updates to avoid gas spikes.

## Dependencies

- Issue #323
- Issue #333
- Issue #325
