# ADR-018: Pull Request and Merge Process

## Status

- **Proposed by:** Ryan on 2026-01-11
- **Accepted on:** 2026-01-11

## Context

As the project grows and gains contributors, we need a documented process for submitting and approving changes. Key concerns:

1. Maintaining a clean, linear commit history on main
2. Ensuring all changes pass CI before merging
3. Consistent code quality through review and automated checks
4. Clear expectations for contributors

## Alternatives Considered

### Alternative 1: Merge Commits

```
main:    A---B---C-------M
              \       /
feature:       D---E---F
```

- **Pros:** Full history preserved, easy to revert entire features, non-destructive
- **Cons:** Non-linear history, harder to bisect, cluttered `git log`

### Alternative 2: Squash and Merge (GitHub)

```
main:    A---B---C---S (single commit with all changes)
```

- **Pros:** Very clean main branch, atomic features, easy reverts, simple contributor workflow
- **Cons:** Loses granular commit history, multiple authors compressed to one commit

### Alternative 3: Rebase + Fast-Forward Only

```
main:    A---B---C---D---E---F (linear)
```

- **Pros:** Linear history, full commit granularity preserved, easy bisect, clear progression
- **Cons:** Requires contributors to rebase before merge, force-push on feature branches, complex local workflow

## Decision

Use **GitHub's "Squash and merge"** for all merges to main.

This gives us:
- **Linear history** on main (no merge commits)
- **Atomic features** (one commit per PR, easy to revert)
- **Simple contributor workflow** (no local squashing or rebasing required)
- **Automatic squashing** handled by GitHub at merge time

## Requirements

### Before Submitting a PR

1. **Pass local checks**: Run `make check` before pushing
   - Formatting: `ruff format` (Python), `cargo fmt` (Rust)
   - Linting: `ruff check` (Python), `cargo clippy` (Rust)
   - Tests: All tests must pass

2. **PR title format**: Use conventional commit format for PR titles:
   ```
   <type>: <description>
   ```
   Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

   The PR title becomes the squash commit message on main, giving us conventional commits on main without requiring them on individual development commits.

### PR Approval Requirements

1. **CI Pipeline**: All workflow gate jobs must pass:
   - **Quality checks** - Linting, security audits, unused dependency checks (all languages)
   - **Language-specific checks** - Unit tests, acceptance tests, builds for each supported language
   - **Documentation** - Docs build successfully

2. **Code Review**: At least one approving review (when multiple contributors)

### Merge Process

1. Contributor pushes feature branch and creates PR
2. CI runs automatically
3. Reviewer approves PR
4. Maintainer clicks "Squash and merge" and edits the commit message
5. GitHub squashes all commits into one and adds to main

### GitHub Repository Configuration

**1. Merge method (Settings > General > Pull Requests)**:
- Enable only "Allow squash merging"
- Disable "Allow merge commits" and "Allow rebase merging"

**2. Branch protection (Settings > Rules > Rulesets)**:

Ruleset name: `Default Branch Ruleset`

Targeting `main` (Default Branch):
- **Require status checks to pass**: All workflow gate jobs (see CI Workflow Structure below)
- **Require pull request reviews**: 1 approving review
- **Dismiss stale reviews on push**: Enabled
- **Require resolution of review threads**: Enabled
- **Require linear history**: Enabled
- **Block deletions**: Enabled
- **Block force pushes**: Enabled
- **Bypass list**: Repository administrators (for emergency merges)

This keeps squash-merge enforced repo-wide while requiring CI to pass for merges to `main`, with admin override for emergencies.

**3. CI Workflow Structure**:

Each workflow has a gate job (named `<Category> Complete`) that aggregates all jobs in that workflow. The ruleset requires all gate jobs to pass.

| Workflow | Gate Job | Purpose |
|----------|----------|---------|
| `quality.yml` | `Quality Complete` | Linting, security audits, unused deps (all languages) |
| `docs.yml` | `Docs Complete` | Documentation builds |
| `<lang>.yml` | `<Lang> Complete` | Language-specific tests and builds |

**Adding a new language binding:**

1. Create `.github/workflows/<lang>.yml` with tests, builds, and a gate job named `<Lang> Complete`
2. Update the ruleset to require the new gate job:
   ```bash
   # Add to required_status_checks in the ruleset
   gh api repos/<owner>/<repo>/rulesets/<id> --method PUT ...
   ```
3. Add language-specific quality checks to `quality.yml` if needed

## Completed

- [x] Branch protection rules to configure on GitHub (use Rulesets)
- [x] Set up required status checks

## Rationale

1. **Linear history** makes `git log`, `git bisect`, and understanding project evolution straightforward
2. **Squash at merge** simplifies the contributor workflow - no need to learn rebase
3. **CI gates** catch issues before they reach main
4. **Consistent formatting** reduces noise in diffs and reviews
5. **One commit per feature** makes reverting atomic and clean

## Trade-offs Accepted

- **Lost granular history** in exchange for **simpler workflow**
- **Squash at merge** means individual commits aren't preserved on main
- **Stricter process** in exchange for **higher code quality**
- **No signed commit requirement**: With squash-and-merge, individual commit signatures don't survive anyway. Accountability is provided by PR metadata (author, reviewers) and GitHub's audit log.

## Migration

N/A - This documents the process going forward.

## Consequences

- **Positive:** Clean, linear, bisectable history on main
- **Positive:** Consistent code quality through automated checks
- **Positive:** Simple contributor workflow (no rebasing required)
- **Positive:** Easy to revert entire features (single commit)
- **Negative:** Granular commit history lost on main (preserved in PR)
- **Neutral:** Maintainers responsible for final merge and commit message
