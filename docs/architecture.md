# Architecture

How `sonda` turns a handful of standard Linux tools into one frozen JSON
document — and the checklist for adding a new probe without breaking the
contract.

## Flow

```text
sonda (main.rs)
  └─ run::run_scan()
       ├─ probe::core    os-release · uname · lscpu · /proc/meminfo · /proc/swaps
       ├─ probe::pci     lspci -k -m + lspci -k          (dual-source merge)
       ├─ probe::usb     lsusb
       ├─ probe::storage lsblk -l
       ├─ probe::net     ip -br link
       ├─ probe::rfkill  rfkill list
       ├─ probe::dmi     dmidecode -s (nullable)
       └─ model::Output  → stdout (JSON | --compact | --summary)
```

The scan never fails the process: missing information stays `null`, probe
failures land in `errors[]` (see [contract.md](contract.md) for the exact
policy).

## Hard conventions

| Convention | Why |
|------------|-----|
| Every external command runs through `Command::new("bash")` + `-c` | Family requirement shared with pkgq: one single execution path, no bypassing. |
| Locale-sensitive commands run as `LC_ALL=C <cmd>` | `lscpu` translates its labels («Socket(s)» on a Spanish system); text parsing depends on the exact English keys. |
| `/proc` and `/etc` files are read directly with `std::fs` | They are files, not commands; going through bash would only add failure modes. |
| Parsers are pure functions over `&str` | They are tested against embedded fixtures and never spawn processes in tests. |
| A probe never panics | Unreadable file → null; missing binary → silent skip; failing command → `ProbeError` in `errors[]`. |

## Module map

| Module | Responsibility |
|--------|----------------|
| `main.rs` | CLI wiring, output modes (pretty / `--compact` / `--summary`), exit codes. |
| `cli.rs` | clap definition: `--compact`, `--summary`. |
| `model.rs` | Contract v1 types + field-order contract test. |
| `shell.rs` | `bash -c` adapter: `run`, `which`, `quote`, `ShellError`. |
| `timefmt.rs` | std-only RFC3339 UTC (civil-from-days), no chrono. |
| `run.rs` | Orchestration: calls every probe, assembles `Output`. |
| `summary.rs` | Human renderer for `--summary` (vendor short names, iGPU heuristic, human sizes). |
| `probe/core.rs` | os-release, `uname -snrmo`, lscpu, meminfo, zram swaps. |
| `probe/pci.rs` | PCI devices with kernel driver state. |
| `probe/usb.rs` | USB devices. |
| `probe/storage.rs` | Block devices (top-level disks). |
| `probe/net.rs` | Network links. |
| `probe/rfkill.rs` | Radio kill switches. |
| `probe/dmi.rs` | Board/BIOS scalar queries (root-only). |

## Why lspci uses two sources

`lspci -k -m` (machine-readable) quotes class, vendor and device as separate
fields — no guessing where the vendor name ends. But on current lspci
versions it does **not** include the kernel driver, and the driver in use is
the gold diagnostic field. So `probe/pci.rs` runs both:

1. `LC_ALL=C lspci -k -m` → slot, class, vendor, device (quoted-field
   scanner in `QuotedTokens`).
2. `LC_ALL=C lspci -k` → `Kernel driver in use:` and `Kernel modules:` per
   slot, merged back into the machine-readable entries by slot address.

Both parsers are fixture-tested against real captured output.

## Why lsblk parses right-to-left

Flat `lsblk -bno NAME,MODEL,SIZE,TYPE,TRAN,RM -l` rows pad columns with
spaces and both MODEL and TRAN may be **empty** (`zram0   4294967296 disk 0`).
Splitting left-to-right is ambiguous; consuming tokens right-to-left is not:
`RM` (0/1), then TRAN only if the token is a known transport (peek, do not
pop — a naive pop once swallowed the TYPE token and silently dropped zram
from a live scan), then TYPE, SIZE, and the remainder is NAME plus optional
MODEL words.

## Adding a new probe

1. **Contract first.** Decide the section shape in `model.rs`: every field
   nullable unless truly always-present, `Option<T>` for anything a system
   can lack. Field order additions go at the end of the section and update
   the order test.
2. **Parser before probe.** Write `parse_<tool>(text: &str) -> ...` as a
   pure function plus fixtures copied from a real machine (not invented
   data — real output has surprises like empty TRAN columns and localized
   labels).
3. **Gate and error policy.** `shell::which(binary)` false → return empty,
   silently. `shell::run` failure → push `ProbeError { probe: "<name>", .. }`
   into the caller's `errors` vec. Never `unwrap` probe results.
4. **Locale.** If the tool prints labels you match on, run it as
   `LC_ALL=C <cmd>`. Prefer formats without labels (`-b`, `-no`, `-m`)
   over parsing human tables.
5. **Wire into `run::run_scan`** in probe order; keep the section ordering
   stable — the contract test will catch an ordering slip.
6. **Document.** Add the section to `docs/contract.md` (top-level table +
   semantics) and, if user-facing at a glance, to `summary.rs`.
7. **Verify live.** `cargo test` proves the parsers; only a real run
   (`cargo run -- --compact | jq .<section>`) proves the probe — the zram
   TRAN bug passed unit tests and was caught by a live scan.

## Testing

| Layer | What | Where |
|-------|------|-------|
| Unit (45) | Pure parsers against real fixtures; serialization and field-order contract; timefmt known values. | `#[cfg(test)]` in each module. |
| Integration (3) | Real binary: JSON shape, single-line `--compact`, `--summary` lines. | `tests/cli.rs` via `CARGO_BIN_EXE_sonda`. |
| Live | Full scan on real hardware before every release. | Manual, recorded in `odd/tasks/*.md`. |

Environment-dependent assertions are forbidden in tests: integration tests
check structure (`command == "scan"`, arrays are arrays), never values.
