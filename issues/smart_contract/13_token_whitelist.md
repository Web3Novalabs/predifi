# Issue 312: Implement Whitelist for Allowed Betting Tokens

## Description

Limit the tokens that can be used for betting to a predefined whitelist managed by the contract administrators. This prevents the use of malicious or unsupported tokens as collateral.

## Tasks

- Create a storage key for the token whitelist.
- Implement admin functions to add/remove tokens from the whitelist.
- Update `create_pool` to check the `token` address against this list.

## Dependencies

- Issue #303
- Issue #307
