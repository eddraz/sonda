//! Core system probes: distro, kernel/arch/hostname, CPU, memory (incl. zram).

use crate::model::{Cpu, Memory, Os, ProbeError};
use crate::probe::read_trim;
use crate::shell;

/// Distro identity from /etc/os-release.
pub(crate) fn os_info() -> Os {
    let distro = read_trim("/etc/os-release").and_then(|t| parse_os_release(&t));
    // GNU coreutils prints fields in a fixed order: sysname, nodename,
    // release, machine, os. Locale-independent labels (bare values).
    let uname = shell::run("uname -snrmo")
        .ok()
        .and_then(|t| parse_uname(&t));
    uname.unwrap_or_default().with_distro(distro)
}

/// CPU summary via `lscpu`. Locale is pinned to C: lscpu translates its
/// labels and text parsing depends on the exact English keys.
pub(crate) fn cpu_info(errors: &mut Vec<ProbeError>) -> Cpu {
    if !shell::which("lscpu") {
        return Cpu::default();
    }
    match shell::run("LC_ALL=C lscpu") {
        Ok(text) => parse_lscpu(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "lscpu".into(),
                message: e.to_string(),
            });
            Cpu::default()
        }
    }
}

/// Memory totals from /proc/meminfo plus zram devices from /proc/swaps.
pub(crate) fn memory_info() -> Memory {
    let mut memory = read_trim("/proc/meminfo")
        .map(|t| parse_meminfo(&t))
        .unwrap_or_default();
    memory.zram_total_bytes = read_trim("/proc/swaps").and_then(|t| parse_swaps_zram(&t));
    memory
}

/// Extract `PRETTY_NAME` (falling back to `NAME`) from os-release content.
fn parse_os_release(text: &str) -> Option<String> {
    let pick = |key: &str| {
        text.lines().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
        })
    };
    pick("PRETTY_NAME")
        .or_else(|| pick("NAME"))
        .filter(|s| !s.is_empty())
}

/// Parse `uname -snrmo` output: `Linux fixy 6.12.107+deb13-amd64 x86_64 GNU/Linux`.
fn parse_uname(text: &str) -> Option<Os> {
    let fields: Vec<&str> = text.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    Some(Os {
        distro: None,
        hostname: Some(fields[1].to_owned()),
        kernel: Some(fields[2].to_owned()),
        arch: Some(fields[3].to_owned()),
    })
}

impl Os {
    fn with_distro(mut self, distro: Option<String>) -> Self {
        self.distro = distro;
        self
    }
}

/// Parse `LC_ALL=C lscpu` key/value lines into a [`Cpu`].
fn parse_lscpu(text: &str) -> Cpu {
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == key).then(|| v.trim().to_owned())
        })
    };
    let cores_per_socket = value("Core(s) per socket").and_then(|v| v.parse::<u32>().ok());
    let sockets = value("Socket(s)").and_then(|v| v.parse::<u32>().ok());
    let cores = cores_per_socket.map(|c| c * sockets.unwrap_or(1));
    Cpu {
        model: value("Model name").filter(|s| !s.is_empty()),
        vendor: value("Vendor ID").map(|v| match v.as_str() {
            "AuthenticAMD" => "AMD".to_owned(),
            "GenuineIntel" => "Intel".to_owned(),
            other => other.to_owned(),
        }),
        cores,
        threads: value("CPU(s)").and_then(|v| v.parse().ok()),
        max_mhz: value("CPU max MHz").and_then(|v| v.parse().ok()),
    }
}

/// Parse /proc/meminfo totals (values are kB) into bytes.
fn parse_meminfo(text: &str) -> Memory {
    let field = |key: &str| {
        text.lines().find_map(|line| {
            let (k, rest) = line.split_once(':')?;
            (k.trim() == key).then(|| {
                rest.split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|kb| kb * 1024)
            })
        })?
    };
    Memory {
        total_bytes: field("MemTotal"),
        available_bytes: field("MemAvailable").or_else(|| field("MemFree")),
        swap_total_bytes: field("SwapTotal"),
        zram_total_bytes: None,
    }
}

/// Sum swap sizes of zram devices from /proc/swaps (sizes are kB); None when
/// zram is not in use.
fn parse_swaps_zram(text: &str) -> Option<u64> {
    let total: u64 = text
        .lines()
        .filter(|line| !line.starts_with("Filename"))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields.len() >= 3 && fields[0].contains("zram"))
                .then(|| fields[2].parse::<u64>().ok())
                .flatten()
        })
        .sum();
    (total > 0).then(|| total * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pretty_name_quoted() {
        let text = "PRETTY_NAME=\"Debian GNU/Linux 13 (trixie)\"\nNAME=\"Debian GNU/Linux\"\n";
        assert_eq!(
            parse_os_release(text).as_deref(),
            Some("Debian GNU/Linux 13 (trixie)")
        );
    }

    #[test]
    fn falls_back_to_name_when_pretty_missing() {
        let text = "NAME=\"Arch Linux\"\nID=arch\n";
        assert_eq!(parse_os_release(text).as_deref(), Some("Arch Linux"));
        assert_eq!(parse_os_release("ID=arch\n"), None);
    }

    #[test]
    fn parses_uname_fields() {
        let os = parse_uname("Linux fixy 6.12.107+deb13-amd64 x86_64 GNU/Linux\n").unwrap();
        assert_eq!(os.hostname.as_deref(), Some("fixy"));
        assert_eq!(os.kernel.as_deref(), Some("6.12.107+deb13-amd64"));
        assert_eq!(os.arch.as_deref(), Some("x86_64"));
        assert_eq!(os.distro, None);
    }

    #[test]
    fn rejects_short_uname_output() {
        assert_eq!(parse_uname("Linux x86_64"), None);
    }

    const LSCPU: &str = "CPU(s):                                  8
Vendor ID:                               AuthenticAMD
Model name:                              AMD Ryzen 3 5300U with Radeon Graphics
Core(s) per socket:                      4
Socket(s):                               1
CPU(s) scaling MHz:                      77%
CPU max MHz:                             3900.0000
";

    #[test]
    fn parses_lscpu_core_fields() {
        let cpu = parse_lscpu(LSCPU);
        assert_eq!(
            cpu.model.as_deref(),
            Some("AMD Ryzen 3 5300U with Radeon Graphics")
        );
        assert_eq!(cpu.vendor.as_deref(), Some("AMD"));
        assert_eq!(cpu.cores, Some(4));
        assert_eq!(cpu.threads, Some(8));
        assert_eq!(cpu.max_mhz, Some(3900.0));
    }

    #[test]
    fn lscpu_exact_key_matching_ignores_prefixed_keys() {
        // "CPU(s) scaling MHz" must not be taken for "CPU(s)".
        let cpu = parse_lscpu("CPU(s) scaling MHz: 77%\n");
        assert_eq!(cpu.threads, None);
    }

    #[test]
    fn lscpu_multiplies_sockets_when_present() {
        let cpu = parse_lscpu(
            "CPU(s): 32\nCore(s) per socket: 8\nSocket(s): 4\nVendor ID: GenuineIntel\n",
        );
        assert_eq!(cpu.cores, Some(32));
        assert_eq!(cpu.vendor.as_deref(), Some("Intel"));
    }

    #[test]
    fn lscpu_garbage_yields_nulls() {
        let cpu = parse_lscpu("nothing useful here\n");
        assert_eq!(cpu.model, None);
        assert_eq!(cpu.cores, None);
        assert_eq!(cpu.threads, None);
    }

    #[test]
    fn parses_meminfo_kb_fields() {
        let mem = parse_meminfo(
            "MemTotal:       11555932 kB\nMemAvailable:    5233016 kB\nSwapTotal:      16069624 kB\nHugePages_Total:       0\n",
        );
        assert_eq!(mem.total_bytes, Some(11_833_274_368));
        assert_eq!(mem.available_bytes, Some(5_358_608_384));
        assert_eq!(mem.swap_total_bytes, Some(16_455_294_976));
    }

    #[test]
    fn meminfo_missing_available_falls_back_to_free() {
        let mem = parse_meminfo("MemTotal: 1000 kB\nMemFree: 250 kB\n");
        assert_eq!(mem.available_bytes, Some(256_000));
    }

    #[test]
    fn sums_zram_swaps_in_bytes() {
        let text = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n/dev/zram0                              partition\t4194300\t\t4193500\t\t100\n/dev/nvme0n1p3                          partition\t11875324\t3631928\t\t-2\n";
        assert_eq!(parse_swaps_zram(text), Some(4_294_963_200));
    }

    #[test]
    fn no_zram_yields_none() {
        let text = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n/dev/sda2                               partition\t4194300\t0\t\t-2\n";
        assert_eq!(parse_swaps_zram(text), None);
    }
}
