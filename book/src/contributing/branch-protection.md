# Branch Protection and Required CI Checks

This page defines the required status checks and protection policy for `main`.

## Branch Protection Policy
Enable branch protection for `main` with:
- Require pull request before merge.
- Require approvals (minimum 1; maintainers may set higher).
- Require conversation resolution before merge.
- Require status checks to pass before merge.
- Restrict force pushes and branch deletions.

## Required Status Checks
Configure these CI checks as required:
- `Rust Check and Test`
- `Spec Tools Check and Test`
- `Grammar Generate and Test`
- `Generated Artifacts Up To Date`

These checks are defined in `.github/workflows/ci.yml`.

## Optional Hardening
- Require branches to be up to date before merging.
- Enable merge queue if PR volume increases.
- Restrict who can dismiss stale reviews.

## Operational Rule
If required checks fail:
- Do not bypass protection.
- Fix the issue or revert the breaking change.
- Re-run checks until green.
