# Security Policy

## Supported Versions

The table below lists which versions of PreDiFi currently receive security fixes.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| older branches | :x:       |

We recommend always using the latest code from the `main` branch. Fixes are not back-ported to older branches unless a release is explicitly tagged.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

To report a vulnerability privately, send an email to:

```
security@predifi.xyz
```

Include the following in your report:

- A clear description of the issue and the potential impact
- The component or file(s) affected (contract, backend, frontend, etc.)
- Steps to reproduce or a proof-of-concept demonstrating the problem (without including live exploit code)
- Any suggested mitigation if you have one

If you are unsure whether an issue qualifies as a security vulnerability, err on the side of caution and email us anyway.

### GitHub Private Security Advisories

As an alternative to email, you can open a [private security advisory](https://github.com/PrediFi/predifi/security/advisories/new) directly on this repository. GitHub keeps advisory drafts confidential until we choose to publish them.

## Response Timeline

| Milestone                             | Target time      |
| ------------------------------------- | ---------------- |
| Acknowledgement of your report        | Within **48 hours** |
| Initial triage and severity assessment | Within **5 business days** |
| Status update (fix in progress / won't fix / needs more info) | Within **10 business days** |
| Patch release or advisory publication  | Depends on severity — critical issues are prioritised |

We will keep you informed at each stage. If you have not received an acknowledgement within 48 hours, please follow up by replying to your original email.

## Disclosure Policy

We follow a **coordinated disclosure** model:

1. Reporter submits a vulnerability privately.
2. We triage, confirm, and develop a fix.
3. We agree on a disclosure date with the reporter (typically 90 days from the initial report, or sooner if a fix is ready).
4. We publish a GitHub Security Advisory and release the patch simultaneously.
5. Credit is given to the reporter unless they prefer to remain anonymous.

We ask that you do not publicly disclose the vulnerability until a patch has been released or the agreed disclosure date has passed.

## Scope

The following are in scope for vulnerability reports:

- Smart contracts in the `contract/` directory
- Backend API in the `backend/` directory
- Frontend application in the `frontend/` directory
- Infrastructure-as-code in the `terraform/` directory

The following are out of scope:

- Third-party dependencies (please report those to the relevant upstream project)
- Issues already publicly known or previously reported
- Theoretical vulnerabilities with no demonstrated impact

## Legal

We will not take legal action against researchers who act in good faith, make a reasonable effort to avoid privacy violations and service disruption, and follow this policy. We consider responsible security research a valuable contribution to the project.
