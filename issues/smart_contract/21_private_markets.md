# Issue 320: Implement Private/Invite-Only Prediction Markets

## Description

Introduce the ability to create private markets that are only accessible to users with a specific invite or those who are on a whitelist provided by the creator. This is useful for community-specific events.

## Tasks

- Add a `private` flag and `whitelist_key` to the `Pool` struct.
- Implement logic in `place_prediction` to check if a user is authorized for a private market.
- Add admin functions for creators to manage their private market whitelists.

## Dependencies

- Issue #305
- Issue #303
- Issue #302
