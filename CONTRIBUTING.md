# Contributing to PrediFi

Welcome to PrediFi! We appreciate your interest in contributing to the project.

This root guide provides an overview of how to contribute across the entire repository. PrediFi consists of multiple components, including the Rust backend and Soroban smart contracts.

---

## 🍴 How to Fork and Clone

1. **Fork the Repository**: Click the **Fork** button at the top-right of the repository page on GitHub.
2. **Clone your Fork**:
   ```bash
   git clone https://github.com/YOUR-USERNAME/predifi.git
   cd predifi
   ```
3. **Set Up Remote**:
   ```bash
   git remote add upstream https://github.com/predifi/predifi.git
   ```

---

## 🌿 Branch Naming

Create a feature or bugfix branch off `main` before making your changes:

- `feat/feature-name` (e.g. `feat/batch-claim`)
- `fix/issue-description` (e.g. `fix/issue-1648`)
- `docs/topic-name` (e.g. `docs/issue-templates`)

---

## 💬 Commit Message Style

We follow Conventional Commits standard practices. Scope your commit message appropriately based on the area of code modified:

- **General**: `fix: resolve race condition in worker`
- **Contract**: `feat(contract): implement batch prediction claiming` or `docs(contract): document create_pool`
- **Backend**: `feat(backend): add redis cache layer`
- **Frontend**: `fix(frontend): adjust loading skeleton component`
- **Docs**: `docs: add comprehensive API guide`

---

## 📬 How to Open a Pull Request

1. **Keep Branches Updated**: Rebase or update your branch against upstream `main`:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```
2. **Run Tests & Linters**: Make sure all checks pass locally before opening a PR.
3. **Push to Your Fork**:
   ```bash
   git push origin feat/your-branch-name
   ```
4. **Create Pull Request**: Open a PR against the `main` branch. Fill out the pull request template completely and link any relevant issues.

---

## 📚 Per-Area Contributing Guides

For specific setup, building, testing, and linting instructions for individual sub-projects, please refer to:

- ⚙️ **Backend Guide**: [CONTRIBUTING_BACKEND.md](CONTRIBUTING_BACKEND.md)
- 📜 **Smart Contracts Guide**: [contract/CONTRIBUTING.md](contract/CONTRIBUTING.md)
