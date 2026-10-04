# GitHub collaboration and maintenance

This document defines the repository's collaboration baseline approved by the
maintainer on 2026-10-04. [ADR 0011](../decisions/0011-github-collaboration.md)
records the decision. Source configuration lives in `.github/`; GitHub repository
settings and active rulesets are also inspected after applying changes.

## Branches, reviews and merges

`main` is the integration branch. Work happens on feature branches in the existing
checkout, and all changes enter through a PR. Only squash merge is enabled.
Merged repository branches are deleted automatically. Auto-merge may be requested
on an authorized PR, but it still waits for required rules; dependency PRs are
never automatically approved or merged by a workflow.

Two independent branch rulesets keep approval policy separate from invariant checks:

- **Main integrity:** no bypass actors. Require a PR, current base, resolved
  conversations, `CI required` and `Security required` checks from GitHub Actions,
  linear history, and prohibit deletion and force pushes.
- **Maintainer review:** require one approval from the code owner, dismiss stale
  approvals and require approval of the latest reviewable push. `CODEOWNERS`
  assigns every path, including itself and workflows, to `@wanquanY`.
  The sole repository administrator has GitHub's **PR-only** exception to this
  approval rule, so an owner-authored PR does not require impossible self-approval.
  This exception cannot skip the separate main-integrity rules or permit a direct push.

External contributions require maintainer review. The maintainer-only exception
is visible in GitHub's rules and PR history; it is not a bot approval or an
independent second review. When a second maintainer is available, remove this
exception and require independent approval for everyone. Do not grant it to bots.

Changes to rules, permissions, ownership or workflow privileges require explicit
maintainer review. An administrator can edit GitHub settings by definition;
workflow configuration cannot remove that administrative authority.

## Required automation

Every PR, including documentation changes, runs CI without path filters that could
leave required checks pending. `CI required` fails if any required job fails,
is cancelled or unexpectedly skips. Dependency review is mandatory on PRs and
rejects new high/critical known vulnerabilities.

CI provisions verified Linux native components from the existing source locks and
runs [the current gate runner](../implementation/current-verification.md), including
Rust, contracts, all TypeScript clients, native workers, MCP profiles and document
Native/WASM parity. README commands and template hashes are checked separately.
Native cache reuse still passes the component hash and source checks in build scripts.

CodeQL scans Actions, C/C++, JavaScript/TypeScript, Python and Rust on PRs, `main`
and weekly. `Security required` requires every language analysis to complete.
Code scanning rules additionally block high/critical security findings and errors.
These checks do not replace Office/WPS, full renderer, product or release acceptance.

Actions use full commit pins, read-only default permissions, bounded jobs and
checkout without persistent credentials. Only CodeQL receives `security-events: write`.
No `pull_request_target` or privileged `workflow_run` executes PR code. All external
contributors require approval before their workflows run. Runners are GitHub-hosted;
public contributions must never execute on a maintainer's personal machine.

## Security and maintenance

Secret scanning and push protection are enabled. Dependabot alerts, security
updates and weekly version-update PRs cover Cargo workspaces, pnpm, Actions,
CI Python requirements and Docker build inputs. Updates still need CI and review.
Report vulnerabilities using [SECURITY.md](../../SECURITY.md).

Issues use bug/feature forms. Discussions handle usage questions. PRs describe
behavior, validation and redistribution rights. Conduct is governed by the
[community policy](../../CODE_OF_CONDUCT.md).

Version tags matching `v*` cannot be modified or deleted. Only repository
administrators may create them. There is no automatic package publishing or
production deployment. Release approval is separate from merging a PR.

## Local reproduction and ongoing verification

Use the commands in the workflows and current verification guide. CI-only
provisioning helpers live in `tools/ci`; they call the existing component builders.
Reports are retained as Actions artifacts for 14 days and include actual failures.
Run `python3 tools/ci/check_github.py` with an authenticated administrator to compare
GitHub against `.github/repository-settings.json` and `.github/rulesets/*.json`.
The audit is read-only and rejects unexpected bypass actors. Inspect settings after policy changes;
editing this document alone does not configure GitHub.
