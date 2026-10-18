# Contract Upgrade Procedure

This document describes who can upgrade the PrediFi Soroban contracts, what
delay (if any) applies, how a pending upgrade can be inspected, and how to roll
back. It is derived from the code in
[`predifi-contract/src/admin.rs`](../contract/contracts/predifi-contract/src/admin.rs)
(`upgrade_contract`, `migrate_state`, `pause` / `unpause`) and
[`access-control/src/lib.rs`](../contract/contracts/access-control/src/lib.rs)
(`assign_role`, `revoke_role`, `propose_new_admin`, `accept_admin_role`).

## 1. Upgrade authority

| Contract | Upgrade entry point | Authority required |
|---|---|---|
| `predifi-contract` | `upgrade_contract(admin, new_wasm_hash)` | `admin` must `require_auth()` **and** hold the `Admin` role (role `0`) in the access-control contract |
| `predifi-contract` | `migrate_state(admin)` | Same: `Admin` role, contract not paused |
| `access-control` | *none* | The access-control contract exposes **no** upgrade function; its Wasm cannot be replaced after deployment |

The `Admin` role is resolved by a cross-contract call to the access-control
contract (`require_admin_role` → `require_role(.., 0)`). Failed attempts emit an
`UnauthorizedAdminAttemptEvent`, which indexers can alert on.

In the access-control contract there is exactly one *current admin* (the address
stored under `DataKey::Admin`). Additional addresses can be granted the `Admin`
role with `assign_role`, and each of those can also call `upgrade_contract` on
`predifi-contract`. Keep the number of `Admin` role holders minimal, and prefer a
multisig account as the admin.

Changing who the authority is:

1. `propose_new_admin(current_admin, new_admin)`
2. `accept_admin_role(new_admin)` (two-step, recommended) — or the legacy
   one-step `transfer_admin`.

The current admin's `Admin` role can **not** be removed with `revoke_role` /
`revoke_all_roles` (both return `AdminError`); hand over first, then revoke the
former admin.

## 2. Delay and attestation

**There is no timelock and no attestation on contract upgrades.** A single
`upgrade_contract` transaction signed by an `Admin` takes effect immediately in
that transaction. No second signer, approval window or on-chain attestation of
the new Wasm is checked by the contract.

Compensating controls (operational, not enforced on-chain):

- Use a multisig / threshold account as the `Admin`.
- Announce the Wasm hash and audit/diff to users before executing.
- Optionally `pause` the contract first (see §4) so no user funds move mid-upgrade.

Only *protocol fee changes* are timelocked (`FEE_CHANGE_TIMELOCK_SECONDS`, via
`set_fee_bps` → `apply_fee_bps`); this delay does **not** apply to upgrades.

## 3. Inspecting an upgrade

Because there is no on-chain "pending upgrade" state (the upgrade is atomic),
inspection happens **before** the transaction is submitted and by observing the
result afterwards.

Before upgrading:

1. Build the candidate Wasm reproducibly and record its hash
   (`stellar contract build`, then `sha256sum` of the `.wasm`).
2. Install the Wasm without upgrading: `stellar contract upload --wasm <file>`
   returns the `new_wasm_hash`. Uploading alone does not change the deployed
   contract.
3. Review the diff against the deployed code and simulate the upgrade
   transaction (`--simulate` / `stellar contract invoke ... --build-only`).
4. Confirm the current version with `get_version()`.

After upgrading:

- `UpgradeEvent { admin, new_wasm_hash }` and
  `ContractUpgradedEvent { old_version, new_version, upgraded_by }` are emitted.
  Compare `new_wasm_hash` with the hash announced beforehand.
- `get_version()` returns `old_version + 1` (stored in instance storage under
  `DataKey::Version`).

## 4. Procedure

1. `pause(admin)` — blocks state-changing user operations while the upgrade runs.
2. `upgrade_contract(admin, new_wasm_hash)` — swaps the Wasm and bumps the version.
3. `migrate_state(admin)` — run any post-upgrade migration (a no-op unless the
   release notes say otherwise; not callable while paused, so `unpause` first if
   the migration requires it, or run migrations that do not require the pause flag).
4. Smoke-test read-only getters (`get_version`, `get_contract_info`, `get_fee_config`).
5. `unpause(admin)`.

Note: `upgrade_contract` and `migrate_state` are guarded as described in §1;
`migrate_state` additionally checks the pause flag (`require_not_paused`), so plan
the pause/unpause ordering accordingly.

## 5. Rollback

Rollback is a **forward upgrade to the previous Wasm**; there is no dedicated
rollback function and no automatic revert.

1. Keep the previous release's Wasm hash (it remains installed on the network,
   and can be re-uploaded from the tagged build if needed).
2. `pause(admin)` if the current version is misbehaving.
3. `upgrade_contract(admin, previous_wasm_hash)`.
4. Version numbering is monotonic: rollback still increments `get_version()`
   (`old_version + 1`); track the "logical" version in release notes.
5. Contract **storage is not reverted**. If the faulty version ran a migration or
   wrote data in a new format, the previous Wasm must be able to read it. Do not
   ship a migration without confirming the prior release can still read the migrated
   state (or ship a compensating migration in the rollback build).
6. `unpause(admin)` after verifying with the read-only getters above.

If the `Admin` key is lost or compromised, there is no on-chain recovery beyond
the access-control admin hand-over (§1); protect the admin key accordingly.
