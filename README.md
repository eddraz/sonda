# sonda

PC hardware, OS, and driver scanner for Linux. One scan, structured JSON
(contract v1) — companion to [pkgq](https://github.com/eddraz/pkgq) and
[faro](https://github.com/eddraz/faro).

Sonda shells out to standard system tools (`uname`, `lscpu`, `lspci -k`,
`lsusb`, `lsblk`, `ip -br link`, `rfkill`, `dmidecode`) and reads `/proc` and
`/etc` directly. Its gold field is the **kernel driver in use** per PCI
device, straight from `lspci -k`.

```console
$ sonda --summary
GPU: AMD Lucienne (iGPU) — driver: amdgpu ✓
WiFi: Realtek RTL8821CE — driver: rtw_8821ce ✓
Kernel: 6.12.107+deb13-amd64 · x86_64 · 4 cores / 8 threads · NVMe KIOXIA 256G + zram 4G
```

## Install

```console
$ cargo install sonda-cli    # from crates.io (installs the `sonda` binary)
$ cargo install --path .     # from a local clone
```

Rust 1.74+ (edition 2021). Dependencies: `clap`, `serde`, `serde_json` —
nothing else.

## Usage

```text
sonda                 # full scan, pretty JSON (contract v1)
sonda --compact       # same, single-line JSON
sonda --summary       # human-readable summary, not JSON
```

Exit code is 0 on every successful scan, even when individual probes fail:
failures are recorded in `errors[]`. A missing probe binary (e.g. no `rfkill`
installed) is skipped silently.

## Updating

```console
$ sonda update
```

Compares the running version against the [latest GitHub
release](https://github.com/eddraz/sonda/releases/latest): downloads the
tarball for your platform, and atomically replaces the running binary
(keeping its permissions). Reports the outcome as JSON; `--compact` for a
single line. Exits 0 on success and on "already up to date"; a fatal failure
prints an error JSON and exits 1.

Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`.

## What it probes

| Probe | Source | Needs | If missing |
|-------|--------|-------|------------|
| Distro | `/etc/os-release` | — | null |
| Kernel, arch, hostname | `uname -snrmo` | `uname` | null |
| CPU model/vendor/cores/threads | `lscpu` | `lscpu` | nulls |
| Memory + zram | `/proc/meminfo`, `/proc/swaps` | — | nulls |
| PCI devices + **driver in use** | `lspci -k` (`-k -m` + text) | `lspci` | empty array |
| USB devices | `lsusb` | `lsusb` | empty array |
| Disks (top-level) | `lsblk -l` | `lsblk` | empty array |
| Network links | `ip -br link` | `ip` | empty array |
| rfkill state | `rfkill list` | `rfkill` | empty array |
| Board/BIOS (DMI) | `dmidecode -s` | `dmidecode` + **root** | nulls |

All commands run through `bash -c` with `LC_ALL=C` pinned where labels are
parsed. Missing binaries are skipped silently; a failing command is recorded
in `errors[]` and the scan still exits 0.

## Quick jq recipes

```console
# Every PCI device with its kernel driver (or SIN DRIVER)
$ sonda --compact | jq -r '.pci_devices[] | "\(.slot)  \(.driver_in_use // "no driver")  \(.device)"'

# Driverless devices only — the diagnostic gold
$ sonda --compact | jq '.pci_devices[] | select(.driver_in_use == null)'

# Memory totals in MiB
$ sonda --compact | jq '.memory | with_entries(.value |= (if . == null then null else (. / 1048576 | round) end))'

# Wireless link state
$ sonda --compact | jq '.network_links[] | select(.name | startswith("wl"))'
```

## JSON contract (frozen for v1)

```json
{
  "command": "scan",
  "generated_at": "2026-02-14T10:00:00Z",
  "os": {
    "distro": "Debian GNU/Linux 13 (trixie)",
    "kernel": "6.12.107+deb13-amd64",
    "arch": "x86_64",
    "hostname": "fixy"
  },
  "cpu": {
    "model": "AMD Ryzen 3 5300U with Radeon Graphics",
    "vendor": "AMD",
    "cores": 4,
    "threads": 8,
    "max_mhz": 3900.0
  },
  "memory": {
    "total_bytes": 11833274368,
    "available_bytes": 5358608384,
    "swap_total_bytes": 16455294976,
    "zram_total_bytes": 4294963200
  },
  "pci_devices": [
    {
      "slot": "01:00.0",
      "class": "Network controller",
      "vendor": "Realtek Semiconductor Co., Ltd.",
      "device": "RTL8821CE 802.11ac PCIe Wireless Network Adapter",
      "driver_in_use": "rtw_8821ce",
      "modules": ["rtw88_8821ce"],
      "category": "wireless"
    }
  ],
  "usb_devices": [],
  "storage": [
    {
      "name": "nvme0n1",
      "model": "KBG40ZNV256G KIOXIA",
      "size_bytes": 256060514304,
      "type": "disk",
      "transport": "nvme",
      "removable": false
    }
  ],
  "network_links": [{ "name": "wlo1", "state": "UP" }],
  "rfkill_devices": [],
  "dmi": {
    "available": false,
    "board_vendor": null,
    "board_name": null,
    "bios_version": null
  },
  "errors": []
}
```

### Contract notes

- Field names and order are frozen; a test asserts the order.
- Every unavailable value serializes as `null`, never as an empty string.
- `driver_in_use: null` means no kernel driver is bound (unloaded or missing
  driver) — diagnostically meaningful, not an error.
- `category` buckets PCI devices: `gpu | wireless | ethernet | audio |
  storage | other`.
- `storage` keeps lsblk rows of `type: disk` (including zram devices).
- `dmi` requires root: without it `available` is `false` and the fields are
  `null`. Run `sudo sonda` to fill it.
- `generated_at` is RFC3339 UTC.

## Troubleshooting

- **`dmi` is all nulls** — dmidecode needs root. `sudo sonda` fills it.
- **`rfkill_devices` is empty** — most likely `rfkill` is not installed
  (`apt install rfkill`). Missing binaries are silent skips by design.
- **`driver_in_use` is null for a device that should have a driver** — the
  module may be unloaded or firmware missing. Check the claiming modules:
  `jq '.pci_devices[] | select(.driver_in_use == null)'` and compare with
  `modules`.
- **Weird labels in output?** Not possible for machine output: label-parsing
  probes run with `LC_ALL=C` pinned. Only `--summary` reuses tool-provided
  names (vendor/device strings), which are locale-independent.
- **`cargo install` runs an old binary** — reinstall with
  `cargo install sonda-cli --force`.

## Documentation

- [docs/contract.md](docs/contract.md) — every field of the JSON contract:
  types, nullability semantics, and a full real example.
- [docs/architecture.md](docs/architecture.md) — how the probes work, the
  parsing conventions, and the checklist for adding a new probe.
- [CHANGELOG.md](CHANGELOG.md) — notable changes per version.

## License

MIT
