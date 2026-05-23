# Contributing to DLSSync

DLSSync is an open-source project under the Apache License 2.0. Contributions of any size are welcome, from typo fixes to new launcher integrations.

## Code of conduct

Be respectful, assume good intent, keep discussions on-topic. Personal attacks, harassment, and discriminatory language are not tolerated. Maintainers reserve the right to remove off-topic or abusive content from issues, pull requests, and discussions.

## Reporting bugs

Open an issue at [github.com/xt0n1-t3ch/DLSSync/issues](https://github.com/xt0n1-t3ch/DLSSync/issues) and include:

- DLSSync version (visible in About).
- Windows build (`winver`).
- GPU vendor and driver version.
- Steps to reproduce.
- Expected vs actual behavior.
- Relevant log lines from `%LOCALAPPDATA%\DLSSync\logs\`.

Mark the issue with the appropriate label. Reproduction steps are the single biggest factor in turnaround time.

## Suggesting features

Open an issue prefixed with `Feature:`. Describe the user problem first, the proposed solution second. Mock-ups or screenshots help. Keep one feature per issue.

## Pull requests

1. Fork the repository and create a topic branch off `main`.
2. Keep the change focused. One PR, one concern.
3. Run the quality gates locally (see below). The CI workflow runs the same gates and must pass before review.
4. Open the PR against `main`. Link the relevant issue.
5. Address review feedback by pushing additional commits to the same branch; the maintainer squashes on merge.

## Commit conventions

Conventional Commit prefixes are encouraged for readability in the changelog:

| Prefix | Use for |
|---|---|
| `feat:` | A new user-visible capability. |
| `fix:` | A bug fix. |
| `perf:` | A measurable performance improvement. |
| `refactor:` | A code change that does not alter behavior. |
| `docs:` | Documentation only. |
| `test:` | Tests only. |
| `chore:` | Build, tooling, or housekeeping. |

Keep the subject line under 72 characters. Use the body for context, not a restatement of the diff.

## Coding standards

- Match the existing patterns of the file you are editing.
- Centralize constants, paths, and shared types rather than duplicating across crates.
- Prefer explicit error handling over `unwrap()` outside of tests.
- Public APIs (commands, plugin entry points) document their preconditions and panics.
- Reserve comments for external constraints (license headers, upstream-bug workarounds with links, lint suppressions). Names and types carry the rest.
- Rust code passes `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
- TypeScript code passes `svelte-check` with zero errors and zero warnings.

## Testing

Add or update tests alongside any behavior change. Tests live under each crate's `src/` (unit) or `tests/` (integration). Frontend component tests live under `frontend/tests/`.

The full validator chain:

```sh
pnpm fmt:rust:check
pnpm lint:rust
pnpm --filter dlssync-frontend check
pnpm --filter dlssync-frontend build
cargo check --workspace
cargo test --workspace
```

All six must pass with zero warnings.

## Release process

Releases are tag-driven. Maintainers bump `version` in `Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`, and `frontend/package.json` together, then push a `vX.Y.Z` tag. GitHub Actions builds the Windows installer and portable artifact, signs the updater payload, and publishes the release.

Pre-release tags (`vX.Y.Z-rc.N`) follow the same flow and are marked as pre-release on GitHub.

## License

By submitting a contribution, you agree that it will be licensed under the project's Apache License 2.0. See [`LICENSE`](LICENSE) and the attribution policy in [`NOTICE`](NOTICE).
