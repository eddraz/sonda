# Changelog

All notable changes to this project are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-09-21

### Added

- `sonda update`: self-update from the latest GitHub Release. Compares
  versions, downloads the platform tarball through `bash -c` (curl), validates
  its SHA-256 checksum when available, runs a sanity pre-flight check, and
  atomically replaces the running binary. JSON report; exits 0 on success
  and on "already up to date", 1 on fatal errors. `--summary` is rejected
  with exit 2 when combined with `update`.
- `sonda update --check`: checks if an update is available against GitHub
  releases without downloading or installing.
- CLI accepts optional subcommands without breaking the bare scan:
  `sonda` (scan), `sonda update`.
- Multi-GPU support in `--summary`: all GPUs detected in `pci_devices` are
  rendered with iGPU / dGPU tagging instead of ignoring secondary GPUs.
- Unprivileged DMI discovery: reads `/sys/class/dmi/id` via `std::fs` without
  requiring root or `dmidecode`, falling back to `dmidecode` when sysfs is
  absent.

### Changed

- Scan performance: independent probes run concurrently in `std::thread::scope`
  (std-only, zero dependencies), reducing scan latency from ~100 ms to ~30-70 ms
  while preserving the exact frozen JSON contract v1 field order and deterministic
  error sequence.
- Release workflow: publishes `.sha256` checksum files alongside tarball assets.

## [0.1.0] - 2026-09-21

### Added

- One-shot hardware/OS/driver scan emitting the frozen JSON contract v1
  (pretty by default, `--compact` for single-line output).
- Core probes: distro (`/etc/os-release`), kernel/arch/hostname (`uname`),
  CPU (`lscpu`, locale-pinned with `LC_ALL=C`), memory totals
  (`/proc/meminfo`) and zram swap devices (`/proc/swaps`).
- PCI devices with **kernel driver in use** and claiming modules
  (`lspci -k`, dual-source machine+text parse), classified into `gpu`,
  `wireless`, `ethernet`, `audio`, `storage`, `other`.
- USB devices (`lsusb`), top-level block devices (`lsblk`, zram included),
  network links (`ip -br link`), radio kill switches (`rfkill`).
- Nullable `dmi` section (board/BIOS via `dmidecode`): nulls without root,
  filled when run under sudo.
- `--summary` human renderer: GPU/WiFi driver lines with an iGPU heuristic
  and a kernel line with cores/threads and storage+zram sizes.
- Nullability policy: missing probe binaries are skipped silently; failing
  commands are recorded in `errors[]`; the process always exits 0 on a
  completed scan.
- Packaging: published to crates.io as **`sonda-cli`** (the plain `sonda`
  name was taken); the installed binary is `sonda`. GitHub Release ships
  prebuilt binaries for linux x86_64/aarch64 and macOS x86_64/aarch64 via a
  tag-triggered workflow with a tag-dispatch rebuild path.
