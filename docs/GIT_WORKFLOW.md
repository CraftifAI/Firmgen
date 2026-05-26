# Git Workflow — Firmgen

This document describes how the CraftifAI team works with the [CraftifAI/Firmgen](https://github.com/CraftifAI/Firmgen) repository.

## Branches

| Branch | Purpose | Who merges |
|--------|---------|------------|
| **`main`** | Production-ready, release-quality code | Maintainers only |
| **`stage`** | Shared integration and internal testing | All team members (via PR) |
| **`feature/*`** | Individual work in progress | Author opens PR into `stage` |

```text
main          ← production releases
  ↑
stage         ← shared testing / integration
  ↑
feature/alice-login-fix
feature/bob-esp32-tool
```

## Daily workflow

### 1. Start from the latest `stage`

Always create new work from an up-to-date `stage` branch:

```powershell
git fetch origin
git checkout stage
git pull origin stage
```

### 2. Create your feature branch

Use a clear, consistent name:

```powershell
git checkout -b feature/<your-name>-<short-description>
```

**Examples:**
- `feature/alice-login-fix`
- `feature/bob-esp32-flash-tool`
- `feature/shubham-chat-export`

**Naming rules:**
- Prefix with `feature/`
- Use lowercase and hyphens (no spaces)
- Keep it short but descriptive
- One feature or fix per branch

### 3. Work, commit, and push

```powershell
git add .
git commit -m "Add clear description of what changed and why"
git push -u origin feature/<your-name>-<short-description>
```

**Commit message tips:**
- Use the imperative mood: "Add login retry" not "Added login retry"
- One logical change per commit when possible
- Reference an issue number if applicable: `Fix auth timeout (#42)`

### 4. Open a Pull Request into `stage`

1. Go to [CraftifAI/Firmgen](https://github.com/CraftifAI/Firmgen)
2. Click **Compare & pull request**
3. Set **base** to `stage` (not `main`)
4. Add a short summary of what changed and how to test it
5. Request review from a teammate

### 5. After review and testing, merge into `stage`

- Use **Squash and merge** or **Merge commit** (team preference: squash keeps history clean)
- Delete the feature branch after merge
- Pull the updated `stage` locally before starting your next branch:

```powershell
git checkout stage
git pull origin stage
```

### 6. Release to production (`stage` → `main`)

When `stage` is stable and tested, a maintainer promotes it to production:

```powershell
git checkout main
git pull origin main
git merge origin/stage
git push origin main
```

Or open a PR: **base `main`**, **compare `stage`**.

Only maintainers should merge into `main`.

## Hotfixes (production emergencies)

If production is broken and cannot wait for the normal `stage` cycle:

```powershell
git fetch origin
git checkout main
git pull origin main
git checkout -b hotfix/<short-description>
# fix, commit, push
git push -u origin hotfix/<short-description>
```

1. Open a PR into **`main`**
2. After merge, also merge the same fix into **`stage`** so branches stay aligned:

```powershell
git checkout stage
git pull origin stage
git merge origin/main
git push origin stage
```

## Rules for the team

1. **Branch from `stage`**, never from `main` or someone else's feature branch
2. **All PRs target `stage`** unless it is a hotfix
3. **Pull `stage` before starting** new work
4. **Keep feature branches short-lived** — aim to merge within a few days
5. **Do not push directly to `main` or `stage`** — use pull requests
6. **Do not force-push** to shared branches (`main`, `stage`)

## First-time setup

Clone the repo and check out `stage`:

```powershell
git clone https://github.com/CraftifAI/Firmgen.git
cd Firmgen
git checkout stage
git pull origin stage
```

Install [GitHub CLI](https://cli.github.com/) (optional but helpful):

```powershell
gh auth login
```

## Keeping your branch up to date

If `stage` moved while you were working, rebase before opening a PR:

```powershell
git fetch origin
git checkout feature/your-branch
git rebase origin/stage
git push --force-with-lease
```

Use `--force-with-lease` (not `--force`) so you do not overwrite someone else's pushes.

## Recommended GitHub branch protection

Repository maintainers should configure under **Settings → Branches**:

**`main`**
- Require pull request before merging
- Require at least 1 approval
- Do not allow force pushes
- Restrict who can push (maintainers only)

**`stage`**
- Require pull request before merging
- Do not allow force pushes

## Quick reference

| Task | Command |
|------|---------|
| Update local `stage` | `git checkout stage && git pull origin stage` |
| Start new work | `git checkout -b feature/name-description` |
| Push feature branch | `git push -u origin feature/name-description` |
| Sync with latest `stage` | `git fetch origin && git rebase origin/stage` |
| Release to production | PR: `stage` → `main` (maintainers) |

## Questions?

Ask in your team channel or tag a maintainer on your pull request.
