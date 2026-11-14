# Contributing to Study Planner

Thanks for taking the time to contribute! Study Planner is a local-first desktop app for academic organization. It keeps schedules, subjects, exams, grades, tasks, notes, and spaced repetition in a SQLite vault that never leaves the user's machine.

This guide covers the development setup, the project's non-negotiable constraints, coding conventions, and the review process.

## Contents

- [Before you start](#before-you-start)
- [Project constraints](#project-constraints)
- [Development setup](#development-setup)
- [Useful commands](#useful-commands)
- [Tests](#tests)
- [Coding guidelines](#coding-guidelines)
- [Commit messages](#commit-messages)
- [Pull requests](#pull-requests)
- [Reporting bugs and requesting features](#reporting-bugs-and-requesting-features)
- [Security](#security)
- [License](#license)

## Before you start

1. Read [README.md](README.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and [docs/DECISIONS.md](docs/DECISIONS.md).
2. Read the "Rules to not break the project" section in [docs/CHECKLIST.md](docs/CHECKLIST.md). Those rules are not negotiable.
3. Search existing issues and pull requests before opening a new one.
4. For large features, refactors, or anything that changes the data model, export format, or security boundaries, open an issue first so the scope can be agreed on.
5. Report security vulnerabilities privately, as described in [SECURITY.md](SECURITY.md). Never open a public issue for an unfixed vulnerability.

## Project constraints

These constraints define the project. A change that breaks one of them will not be merged.

- **Local first.** No accounts, no cloud sync, no telemetry, and no background network activity.
- **No network in the webview.** The UI must not use `fetch`, HTTP clients, remote CDNs, remote fonts, or remote icons. The only network-capable component is the isolated updater crate (`crates/study-planner-updater`): Rust-only, opt-in, with a fixed URL and mandatory signature verification.
- **No new permissions without justification.** Do not widen the Tauri capabilities or the CSP to add `fs`, `http`, `sql`, `shell`, `opener`, `dialog`, `notification`, or `updater` access to the frontend. Discuss the need in an issue first.
- **Vault data never enters the repository.** Tests, fixtures, screenshots, and logs must use synthetic data. Never commit a real vault, database, export, or personal attachment.
- **No vault encryption.** The absence of vault encryption is an explicit project decision. Do not introduce implicit encryption or change this assumption without prior discussion.
- **Schema and export compatibility.** SQLite changes require versioned migrations. Changes to the canonical export format require round-trip tests, documentation updates, and a compatibility plan. Never edit a migration that has already been released.
- **License compatibility.** New dependencies must be compatible with MIT OR Apache-2.0. Tauri's transitive MPL-2.0 dependency is a scoped exception documented in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Development setup

### Prerequisites

- **Linux x86_64** (tested on CachyOS/Arch). Windows x64 and macOS arm64 build and update, but do not have local notifications or OS-level code signing.
- **Node.js 22+** and npm.
- **Stable Rust 1.90+** with `cargo`.
- **WebKitGTK 4.1**, GTK 3, libsoup 3, and the rest of Tauri's Linux dependencies.
- Optional on Linux: `libayatana-appindicator3` for the background tray icon.
- Optional: `rg` (ripgrep) for `scripts/audit-network.sh`.

### First build

```bash
git clone https://github.com/wulflo/StudyPlanner.git
cd StudyPlanner
npm install
npm run tauri dev
```

To work on the frontend without the Tauri IPC layer:

```bash
npm run dev
```

## Useful commands

| Command | Purpose |
| :--- | :--- |
| `npm run dev` | Frontend only, in the browser |
| `npm run tauri dev` | Full desktop app |
| `npm run typecheck` | TypeScript checks |
| `npm run lint` | Biome checks |
| `npm run format` | Format the frontend with Biome |
| `npm test` | Run the frontend test suite once |
| `npm run test:watch` | Run frontend tests in watch mode |
| `npm run build` | Type-check and build the frontend |
| `npm run check` | Typecheck + lint + tests + build |
| `npm run check:all` | `check` + Rust fmt + Clippy + Rust tests |
| `./scripts/check.sh` | Full verification, including the Tauri host check and audits |
| `./scripts/audit-network.sh` | Audit network access, permissions, and secrets |
| `./scripts/audit-licenses.sh` | Audit dependency licenses |

Rust-only commands:

```bash
cargo fmt --all
cargo clippy -p study-planner-core -p study-planner-store \
  -p study-planner-transfer -p study-planner-updater --all-targets -- -D warnings
cargo clippy -p study-planner --all-targets -- -D warnings
cargo test -p study-planner-core -p study-planner-store \
  -p study-planner-transfer -p study-planner-updater
cargo test -p study-planner
```

## Tests

- Add tests for every new behavior and every bug fix.
- Frontend tests use Vitest and Testing Library. Rust tests use the standard `cargo test` harness.
- Pay special attention to migrations, backup and restore, export/import round-trips, updater URL validation, and signature verification. These areas handle data integrity and security.
- Before pushing, run `./scripts/check.sh` (recommended) or at least `npm run check:all`.
- CI must pass before a pull request can be merged.

If a check is slow or unavailable on your machine, say so in the pull request and include the checks you did run.

## Coding guidelines

### TypeScript and React

- Keep TypeScript strict mode clean. Avoid `any`; prefer explicit types and domain models.
- Use function components and hooks. Keep state in Zustand stores where the existing code does.
- Never add network calls, remote assets, or direct filesystem access to the frontend.
- Run `npm run format` and `npm run lint` before committing.

### Rust

- Run `cargo fmt --all` and keep Clippy clean with `-D warnings`.
- Keep domain logic in the domain crates. Side effects (filesystem, SQLite, network, OS integration) belong at the edges.
- Prefer explicit errors over panics. Do not include vault contents, secrets, or personal paths in error messages or logs.
- Tauri commands belong in Rust. Do not expose new capabilities to the webview as a shortcut.

### Data and compatibility

- Add a new versioned migration for schema changes. Do not rewrite migrations that have already shipped.
- Preserve the guarantee that `export → import → export` produces identical bytes on the same version.
- Update [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md) or [`docs/EXPORT_FORMAT.md`](docs/EXPORT_FORMAT.md) when their contracts change.

### Documentation and changelog

- Update the relevant files under `docs/` when behavior, architecture, or security boundaries change.
- Add user-visible changes to [CHANGELOG.md](CHANGELOG.md) under `Unreleased`.
- Record architectural decisions in [`docs/DECISIONS.md`](docs/DECISIONS.md).
- Update [docs/CHECKLIST.md](docs/CHECKLIST.md) when a phase item is completed or a process changes.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) in the imperative mood, with a scope when it helps:

```text
feat(ui): add exam countdown
fix(store): preserve backup before migration
docs(snap): document channel promotion
chore(deps): update Tauri
```

Keep commits focused. Reference issues when relevant, for example `Closes #42`.

## Pull requests

1. Create a branch from `main` (fork the repository first if you do not have write access), for example `feat/exam-countdown` or `fix/backup-restore`.
2. Make the change, add tests, and update the documentation and changelog.
3. Run `./scripts/check.sh` and confirm that `scripts/audit-network.sh` passes.
4. Verify that no secrets, real vault data, or personal information are included in the diff.
5. Open the pull request with:
   - a short summary of what changed and why;
   - screenshots or a recording for UI changes;
   - the tests and checks you ran;
   - the issue it closes, if any.
6. Keep the CI green and respond to review comments. The maintainer may squash or rebase commits before merging.
7. Releases are cut by the maintainer from `v*` tags; do not create release tags in a pull request.

## Reporting bugs and requesting features

Open a GitHub issue and include:

- Study Planner version and operating system.
- Steps to reproduce the problem.
- Expected and actual behavior.
- Relevant logs or screenshots, with personal data removed.

Do not attach a real vault, database, export file, signing key, passphrase, or token. Use synthetic data in reproductions.

## Security

Do not open a public issue for a vulnerability. Use GitHub's private advisory form:

**https://github.com/wulflo/StudyPlanner/security/advisories/new**

See [SECURITY.md](SECURITY.md) for scope and reporting details.

## License

By contributing, you agree that your contributions are licensed under the same terms as the project: **MIT OR Apache-2.0**. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for the full texts.
