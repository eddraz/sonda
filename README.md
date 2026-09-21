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
$ cargo install --path .
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

## License

MIT
