# Contributing to MusterOffice

English · [简体中文](CONTRIBUTING.zh-CN.md)

Start with the [README](README.md), [architecture](docs/architecture/overview.md)
and [accepted decisions](docs/decisions/README.md). Questions belong in
[Discussions](https://github.com/wanquanY/MusterOffice/discussions); reproducible
bugs and feature proposals belong in [Issues](https://github.com/wanquanY/MusterOffice/issues).
Report vulnerabilities through the [private security process](SECURITY.md).

## Contribution workflow

1. Discuss substantial architecture, format or dependency changes before implementation.
2. Fork the repository, or create a feature branch if you have write access.
   Keep work in the current checkout unless a separate worktree is explicitly requested.
3. Make a focused change with an explanation, appropriate tests and updated documentation.
4. Open a pull request against `main`, using the provided template. Draft PRs are welcome.
5. Resolve review conversations and let all required checks pass on the current revision.
6. The maintainer reviews external contributions and merges approved changes using squash.
   GitHub removes the merged source branch in this repository automatically.

Direct pushes to `main`, force pushes and deleting protected branches are prohibited.
The branch must be up to date before merge. CI and security checks apply to the
maintainer as well. Current single-maintainer review handling and the exact GitHub
rules are documented in [repository governance](docs/governance/github-collaboration.md).

Use descriptive commit subjects explaining the change. A PR title becomes its
squash commit subject; write it for someone reading history without the discussion.
Keep unrelated changes in separate PRs. Do not rewrite a shared branch without
coordination. Commit signing is welcome but is not currently a required gate.

## Engineering and validation

Keep the computation core independent of accounts, permissions, networks, storage
and product UI. Respect the [engineering guide](AGENTS.md), clear module ownership,
source file size limits and the existing Rust/native/WASM architecture. Fix defects
at their owning layer rather than adding compensating behavior in a consumer.

Follow [current verification](docs/implementation/current-verification.md) for local
commands and native build prerequisites. GitHub CI provisions pinned components
from public upstream sources, then runs that same gate runner. It covers Rust
formatting, Clippy and tests, schemas, TypeScript clients, native workers, MCP
profiles and document native/WASM parity. The README examples and template files
are checked as well. CodeQL and dependency review are separate required gates.

Do not silently skip a failing check or weaken precision, content, editability or
resource limits to obtain a passing result. Record checks you could not run.
Office/WPS interoperability, full renderer parity, product acceptance and release
performance require their own evidence; a green PR does not certify them.

Include a minimal, redistributable input for a bug fix. Remove credentials and
private content. Rendering changes should include previews and describe fonts,
platform and component versions. Contract changes must update the Rust-generated
schemas and TypeScript types through the normal generators.

## Rights and conduct

Original project work is licensed under [Apache-2.0](LICENSE). Contributions are
submitted under that license; retain third-party notices and verify redistribution
rights for code, fonts, templates and assets. There is no separate CLA or mandatory
DCO sign-off at present. See [licensing](docs/governance/licensing.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md).

Release tags and publication are maintainer responsibilities. Merging a PR does
not authorize publishing a package, deploying Musterwork or announcing compatibility.
