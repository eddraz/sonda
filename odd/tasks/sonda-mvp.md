# Feature: sonda-mvp (Rust CLI that scans PC hardware, OS, and drivers)

Status: in progress — MVP implementation
Crate/binary: sonda
Branch: feature/sonda-mvp (branched from unborn master before first commit)

## Goal

Rust CLI that scans the machine it runs on — OS, architecture, kernel, CPU,
memory, PCI devices with kernel drivers in use, USB devices, storage, network
links, rfkill state — and returns structured JSON (contract v1, frozen).
Companion to pkgq and faro (same family, same JSON contract v1 pattern).
Shells out exclusively through `bash -c`; reads `/proc` and `/etc` files
directly. Target platform: Linux (Debian first). Fields that require root
(dmidecode DMI data) are nullable: without root they serialize as `null` and
the tool still exits 0.

## Decisions (user-confirmed)

- Name: `sonda` (family: faro, sonda).
- Output: JSON contract v1 by default (pretty; `--compact` for one line) —
  consistent with pkgq/faro. Human summary (the demo format) via `--summary`.
- Kernel driver in use (from `lspci -k`) is a first-class field: gold for
  driver diagnostics.
- dmidecode-dependent fields: nullable; MVP needs no root.
- Missing probe binaries (lspci, lsusb, ip, rfkill…) are skipped silently;
  only real command failures land in `errors[]`. Exit code stays 0.

## JSON contract (frozen for v1)

```json
{
  "command": "scan",
  "generated_at": "2026-02-14T10:00:00Z",
  "os": {
    "distro": "Debian GNU/Linux 13",
    "kernel": "6.12.107+deb13-amd64",
    "arch": "x86_64",
    "hostname": "host"
  },
  "cpu": {
    "model": "AMD Ryzen 7 5700U with Radeon Graphics",
    "vendor": "AMD",
    "cores": 8,
    "threads": 16,
    "max_mhz": 4316.757
  },
  "memory": {
    "total_bytes": 16777216000,
    "available_bytes": 8388608000,
    "swap_total_bytes": 4294967296,
    "zram_total_bytes": 4294967296
  },
  "pci_devices": [
    {
      "slot": "03:00.0",
      "class": "Network controller",
      "vendor": "Realtek Semiconductor Co., Ltd.",
      "device": "RTL8821CE 802.11ac PCIe Wireless Network Adapter",
      "driver_in_use": "rtw_8821ce",
      "modules": ["rtw88_8821ce"],
      "category": "wireless"
    }
  ],
  "usb_devices": [
    {
      "bus": "001",
      "device": "003",
      "vendor_id": "8087",
      "product_id": "0026",
      "name": "Intel Corp. AX201 Bluetooth"
    }
  ],
  "storage": [
    {
      "name": "nvme0n1",
      "model": "KIOXIA-EXCERIA SATA SSD",
      "size_bytes": 256060514304,
      "type": "disk",
      "transport": "nvme",
      "removable": false
    }
  ],
  "network_links": [{ "name": "wlan0", "state": "UP" }],
  "rfkill_devices": [
    {
      "name": "phy0: Wireless LAN",
      "type": "wlan",
      "soft_blocked": false,
      "hard_blocked": false
    }
  ],
  "dmi": {
    "available": false,
    "board_vendor": null,
    "board_name": null,
    "bios_version": null
  },
  "errors": []
}
```

- Field order is part of the contract (asserted by a test, like pkgq).
- `category`: gpu | wireless | ethernet | audio | storage | other.
- `driver_in_use`: from `lspci -k` "Kernel driver in use"; `null` when absent
  (unloaded driver or no driver) — diagnostically meaningful, not an error.
- `storage.type` copies lsblk TYPE; rows filtered to TYPE=disk. `transport`
  from TRAN (nvme | sata | usb | null). `removable` from RM.
- `memory.zram_total_bytes`: sum of `/proc/swaps` zram entries (null when none).
- `dmi.available=false` + nulls when dmidecode is unavailable/permitted;
  `true` with real values when run under root.
- Timestamp: RFC3339 UTC, std-only implementation (no chrono).
- Dependencies: clap (derive), serde, serde_json — nothing else.

## CLI surface

```
sonda                 # full scan, pretty JSON contract v1
sonda --compact       # same, single-line JSON
sonda --summary       # human summary (demo format), not JSON
```

Demo target for `--summary`:

```
GPU: AMD Lucienne (iGPU) — driver: amdgpu ✓
WiFi: Realtek RTL8821CE — driver: rtw_8821ce ✓
Kernel: 6.12.107+deb13-amd64 · x86_64 · 8 cores · NVMe Kioxia 256G + zram 4G
```

iGPU heuristic: GPU vendor == CPU vendor (AMD/Intel) → append "(iGPU)";
discrete otherwise; no marker when undeterminable. Summary degrades
gracefully when a section is missing.

## Tasks

- [ ] T1: Scaffold crate (Cargo.toml, main.rs, cli.rs) + bash adapter
      (shell.rs: run/which/quote + tests)
- [ ] T2: Contract v1 model (frozen field order test) + core probes:
      os-release, uname, lscpu, /proc/meminfo + /proc/swaps (zram) + tests
- [ ] T3: Device probes: lspci -k parser (drivers, categories), lsusb,
      rfkill + tests
- [ ] T4: Storage (lsblk) + network links (ip -br link) + nullable dmidecode
      + tests
- [ ] T5: run.rs orchestration (errors[], exit 0) + output modes (default
      JSON, --compact, --summary) + summary tests
- [ ] T6: README + integration test (CARGO_BIN_EXE) + full battery
      (fmt, clippy, test, release build)

## Evidence log

- (empty — will record commits and test counts per task)
