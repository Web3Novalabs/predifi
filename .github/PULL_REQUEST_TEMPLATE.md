# Description

Provide a brief summary of the changes and the motivation behind them.

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] CI/CD or internal tool improvement

## How Has This Been Tested?

Please describe the tests that you ran to verify your changes. Provide instructions so we can reproduce.

<!--
Be specific enough that a reviewer can repeat it:
  - commands you ran and whether they passed (`cargo test -p predifi-backend`, `pnpm test`)
  - anything you could NOT verify locally, and why
Saying what was not verified is more useful than leaving it implied.
-->

## Screenshots / Recordings

> [!IMPORTANT]
> **A screenshot or screen recording is REQUIRED for all frontend/UI changes.**
> Please paste your screenshots or recordings below.

<!-- Paste screenshots/recordings here -->

## Deployment Impact

> Tick anything this change requires. If a box is ticked, say what has to happen
> and in what order — a reviewer cannot infer a deployment step from a diff, and
> finding out after merge is the expensive way.

- [ ] **Database migration** — this PR adds or changes a migration in `backend/migrations/`
- [ ] **Configuration change** — a new or changed environment variable, secret, or config file
- [ ] **Infrastructure change** — Terraform or Docker changes needing an apply
- [ ] **Contract deployment** — the Soroban contract must be rebuilt and redeployed
- [ ] **Breaking API change** — existing clients need updating
- [ ] None of the above

<!--
If you ticked a box:
  - Migration: is it reversible? Does it need a backfill? Is it safe to run
    while the old code is still serving traffic?
  - Configuration: what is the new variable, what is its default, and what
    happens if it is missing in production?
  - Contract: which network, and does any stored state need migrating?
-->

## Checklist

- [ ] My code follows the style guidelines of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published in downstream modules

## Related Issues

<!-- `Closes #123` for work that fully resolves an issue, `Refs #123` for partial work. -->
