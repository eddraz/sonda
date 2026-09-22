# JSON contract v1

Every `sonda` scan emits exactly one JSON document: the contract below.
Field names and field order are frozen — a unit test asserts the order, so
consumers can rely on it. Anything that cannot be probed serializes as
`null`, never as an empty string, and the process still exits 0.

## Example

Real output from a Debian 13 laptop (PCI list trimmed to the interesting
rows):

```json
{
  "command": "scan",
  "generated_at": "2026-09-21T18:44:44Z",
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
    "available_bytes": 4712366080,
    "swap_total_bytes": 16455294976,
    "zram_total_bytes": 4294963200
  },
  "pci_devices": [
    {
      "slot": "03:00.0",
      "class": "VGA compatible controller",
      "vendor": "Advanced Micro Devices, Inc. [AMD/ATI]",
      "device": "Lucienne",
      "driver_in_use": "amdgpu",
      "modules": ["amdgpu"],
      "category": "gpu"
    },
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
  "usb_devices": [
    {
      "bus": "001",
      "device": "003",
      "vendor_id": "0bda",
      "product_id": "b00e",
      "name": "Realtek Semiconductor Corp. Bluetooth Radio"
    }
  ],
  "storage": [
    {
      "name": "zram0",
      "model": null,
      "size_bytes": 4294967296,
      "type": "disk",
      "transport": null,
      "removable": false
    },
    {
      "name": "nvme0n1",
      "model": "KBG40ZNV256G KIOXIA",
      "size_bytes": 256060514304,
      "type": "disk",
      "transport": "nvme",
      "removable": false
    }
  ],
  "network_links": [
    { "name": "lo", "state": "UNKNOWN" },
    { "name": "wlo1", "state": "UP" }
  ],
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

## Top-level fields

| Field | Type | Semantics |
|-------|------|-----------|
| `command` | string | Always `"scan"` in v1. |
| `generated_at` | string | RFC3339 UTC timestamp taken when the scan starts. |
| `os` | object | Distro, kernel release, machine arch, hostname. |
| `cpu` | object | lscpu summary; `cores` is physical cores, `threads` logical. |
| `memory` | object | Byte totals from /proc/meminfo; `zram_total_bytes` from /proc/swaps. |
| `pci_devices` | array | One entry per PCI device, in `lspci` order. |
| `usb_devices` | array | One entry per `lsusb` row. |
| `storage` | array | `lsblk` rows of `type: disk` only (includes zram devices). |
| `network_links` | array | `ip -br link` interfaces, loopback included. |
| `rfkill_devices` | array | rfkill blocks; empty when `rfkill` is not installed. |
| `dmi` | object | Board/BIOS data from `/sys/class/dmi/id` (sysfs) or `dmidecode`. |
| `errors` | array | Probe failures only — never missing-binary skips. |

## Nullability policy

| Situation | Result |
|-----------|--------|
| Probe binary not installed (e.g. no `rfkill`) | Section skipped silently: empty array / null fields, **no** `errors[]` entry. |
| Probe binary present but the command fails | `errors[]` entry `{"probe": ..., "message": ...}`; fields stay null/empty. |
| Value absent on an otherwise healthy system | `null` field (e.g. `driver_in_use` with no driver bound). |
| DMI unavailable (no sysfs, dmidecode without root) | `dmi.available = false` and all `dmi` fields `null`. Read from sysfs unprivileged or run `sudo sonda`. |

## Field semantics worth knowing

- **`pci_devices[].driver_in_use`** — the gold field for driver
  diagnostics, taken from `lspci -k` “Kernel driver in use”. `null` means no
  kernel driver is currently bound: unloaded module, missing firmware, or a
  genuinely driverless device. Diagnostically meaningful, not an error.
- **`pci_devices[].modules`** — kernel modules that claim the device
  (“Kernel modules:” in `lspci -k`); useful when `driver_in_use` is null and
  you want to know what *could* claim it.
- **`category`** — coarse bucket derived from the pci.ids class name:
  `gpu` (VGA / Display / 3D), `wireless` (Network controller), `ethernet`,
  `audio`, `storage` (NVMe / SATA / IDE / RAID), otherwise `other`.
- **`cpu.cores` vs `cpu.threads`** — cores = “Core(s) per socket” ×
  “Socket(s)”; threads = `CPU(s)`. A Ryzen 3 5300U reports 4 cores and 8
  threads.
- **`memory.*_bytes`** — exact bytes (meminfo values are kB × 1024).
  `available_bytes` falls back to `MemFree` when `MemAvailable` is absent.
- **`storage[].type` / `transport`** — copied from lsblk `TYPE` / `TRAN`;
  `transport` is null for devices without one (zram). `size_bytes` is exact.
- **`usb_devices` ids** — strings with leading zeros preserved (`"001"`),
  straight from `lsusb`.

## Versioning

- v1 is frozen: no renames, no removals, no reorder. Consumers may parse
  strictly.
- Additions (a new probe section, a new nullable field) will land as a v2
  contract, never silently inside v1.
- The field order is part of the contract and enforced by
  `model.rs::output_field_order_matches_contract`.
