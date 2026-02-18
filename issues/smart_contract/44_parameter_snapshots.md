# Issue 343: Protocol Parameter Snapshotting for Audits

## Description

Maintain a versioned history of protocol parameters (fees, delays, white-lists) in storage. This allows for historical auditing and gives users transparency into what the rules were when they placed their prediction.

## Tasks

- Create a `ParameterHistory` storage structure.
- Update admin functions to record snapshots upon every change.
- Implement high-efficiency reads for historical data.

## Dependencies

- Issue #302
- Issue #303
- Issue #317
