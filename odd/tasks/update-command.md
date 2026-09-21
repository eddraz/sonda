# Feature: update-command (sonda update — self-update from GitHub Releases)

Status: in progress
Branch: feat/update-command (from master 29116f9)

## Goal

`sonda update` self-updates the installed binary from the latest GitHub
Release, mirroring the pkgq self-update pattern: GitHub API for the latest
tag, semver-ish compare against `CARGO_PKG_VERSION`, asset
`sonda-<version>-<target>.tar.gz`, download via curl through `bash -c`,
atomic replace next to the current executable. JSON report on stdout;
exit 0 on success / "already up to date", FAILURE with error JSON on fatal.

## Decisions

- Command name: `update` (user's wording).
- Backward compatible CLI: clap `Option<Subcommand>` — bare `sonda` keeps
  scanning with `--compact`/`--summary`; no breaking restructure (unlike
  pkgq/faro which had to break).
- `--summary` is rejected with exit 2 when combined with `update`.
- No checksum verification in v1 (release.yml does not publish checksum
  assets yet); parity with pkgq's updater. Candidate follow-up: emit
  `.sha256` assets and verify.
- Version bump 0.1.0 → 0.2.0 in the same PR (semver 0.x minor for a new
  feature). Tag + release + crates.io publish happen AFTER merge, on user
  decision.

## Tasks

- [ ] T1: src/update.rs (port from pkgq: target_triple, is_newer, fetch,
      select_asset, download_and_replace, run_update) + test suite
- [ ] T2: cli.rs Option<Subcommand> + main.rs dispatch + summary guard +
      version bump 0.2.0
- [ ] T3: README (Updating section) + CHANGELOG (0.2.0)
- [ ] T4: PR (issue-first, type:feature) + CI green

## Evidence log

- (empty)
