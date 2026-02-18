# Issue 318: Implement Referral Fee Distribution Mechanism

## Description

Enable a referral system where users can invite others to a market. A portion of the protocol fee collected from the referred user is distributed to the referrer.

## Tasks

- Update `place_prediction` to accept an optional referrer address.
- Track referred volume per user for payout calculation.
- Implement logic in `claim_winnings` to distribute the referral cut.

## Dependencies

- Issue #302
- Issue #304
- Issue #317
