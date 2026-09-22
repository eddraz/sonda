//! Human-readable summary renderer (the `--summary` mode).
//!
//! Target shape:
//! ```text
//! GPU: AMD Lucienne (iGPU) — driver: amdgpu ✓
//! WiFi: Realtek RTL8821CE — driver: rtw_8821ce ✓
//! Kernel: 6.12.107+deb13-amd64 · x86_64 · 4 cores / 8 threads · NVMe KIOXIA 256G + zram 4G
//! ```
//! Summary strings are informational: exact values live in the JSON output.

use crate::model::{Output, PciDevice};

pub fn render(out: &Output) -> String {
    let gpus: Vec<&PciDevice> = out
        .pci_devices
        .iter()
        .filter(|d| d.category == "gpu")
        .collect();
    let wifi = out.pci_devices.iter().find(|d| d.category == "wireless");

    let mut lines = Vec::new();
    if gpus.is_empty() {
        lines.push(device_line("GPU", None, true, out));
    } else {
        for gpu in &gpus {
            lines.push(device_line("GPU", Some(gpu), true, out));
        }
    }
    lines.push(device_line("WiFi", wifi, false, out));
    lines.push(kernel_line(out));
    lines.join("\n")
}

fn device_line(label: &str, dev: Option<&PciDevice>, allow_igpu: bool, out: &Output) -> String {
    let Some(dev) = dev else {
        return format!("{label}: not detected");
    };
    let vendor = vendor_short(&dev.vendor);
    let name = device_short(&dev.vendor, &dev.device);
    let gpu_count = out
        .pci_devices
        .iter()
        .filter(|d| d.category == "gpu")
        .count();
    let igpu = if allow_igpu && is_igpu(&dev.vendor, out) {
        " (iGPU)"
    } else if allow_igpu && gpu_count > 1 {
        " (dGPU)"
    } else {
        ""
    };
    let driver = match &dev.driver_in_use {
        Some(driver) => format!("driver: {driver} ✓"),
        None => "driver: none ✗".to_owned(),
    };
    format!("{label}: {vendor} {name}{igpu} — {driver}")
}

/// Short vendor label for summary display.
fn vendor_short(vendor: &str) -> &str {
    if vendor.contains("[AMD") {
        "AMD"
    } else if vendor.starts_with("Intel") {
        "Intel"
    } else if vendor.contains("Realtek") {
        "Realtek"
    } else if vendor.contains("Qualcomm") {
        "Qualcomm"
    } else if vendor.contains("NVIDIA") {
        "NVIDIA"
    } else {
        vendor.split_whitespace().next().unwrap_or(vendor)
    }
}

/// Compact device name: drop the vendor prefix, keep the first model token.
fn device_short<'a>(vendor: &str, device: &'a str) -> &'a str {
    let stripped = device.strip_prefix(vendor).map(str::trim).unwrap_or(device);
    stripped
        .split(" (")
        .next()
        .unwrap_or(stripped)
        .split_whitespace()
        .next()
        .unwrap_or(stripped)
}

/// Heuristic: GPU vendor matching the CPU vendor means integrated graphics.
fn is_igpu(gpu_vendor: &str, out: &Output) -> bool {
    let gpu = vendor_short(gpu_vendor);
    let cpu = out.cpu.vendor.as_deref().unwrap_or("");
    matches!((gpu, cpu), ("AMD", "AMD") | ("Intel", "Intel"))
}

fn kernel_line(out: &Output) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(
        out.os
            .kernel
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
    );
    if let Some(arch) = &out.os.arch {
        parts.push(arch.clone());
    }
    match (out.cpu.cores, out.cpu.threads) {
        (Some(c), Some(t)) => parts.push(format!("{c} cores / {t} threads")),
        (Some(c), None) => parts.push(format!("{c} cores")),
        (None, Some(t)) => parts.push(format!("{t} threads")),
        (None, None) => {}
    }
    let storage = storage_segment(out);
    if !storage.is_empty() {
        parts.push(storage);
    }
    format!("Kernel: {}", parts.join(" · "))
}

/// Primary disk plus zram suffix, e.g. `NVMe KIOXIA 256G + zram 4G`.
fn storage_segment(out: &Output) -> String {
    let primary = out
        .storage
        .iter()
        .find(|s| matches!(s.transport.as_deref(), Some("nvme") | Some("sata")))
        .or_else(|| out.storage.iter().find(|s| s.device_type == "disk"));
    let mut segment = primary
        .map(|s| {
            let transport = match s.transport.as_deref() {
                Some("nvme") => "NVMe".to_owned(),
                Some("sata") => "SATA".to_owned(),
                Some("usb") => "USB".to_owned(),
                Some(other) => other.to_owned(),
                None => String::new(),
            };
            let model = model_short(s).map(|m| format!(" {m}")).unwrap_or_default();
            let size = s
                .size_bytes
                .map(|b| format!(" {}", human_size(b)))
                .unwrap_or_default();
            format!("{transport}{model}{size}").trim().to_owned()
        })
        .unwrap_or_default();
    if let Some(zram) = zram_bytes(out) {
        if !segment.is_empty() {
            segment.push_str(" + ");
        }
        segment.push_str(&format!("zram {}", human_size(zram)));
    }
    segment
}

/// Model without alphanumeric noise (tokens containing digits), falling back
/// to the raw model or the device name.
fn model_short(storage: &crate::model::StorageDevice) -> Option<String> {
    let model = storage.model.as_deref()?;
    let cleaned: Vec<&str> = model
        .split_whitespace()
        .filter(|t| !t.chars().any(|c| c.is_ascii_digit()))
        .collect();
    let joined = cleaned.join(" ");
    let chosen = if cleaned.is_empty() {
        model
    } else {
        joined.as_str()
    };
    (!chosen.is_empty()).then(|| chosen.to_owned())
}

fn zram_bytes(out: &Output) -> Option<u64> {
    out.memory.zram_total_bytes.or_else(|| {
        out.storage
            .iter()
            .find(|s| s.name.starts_with("zram"))
            .and_then(|s| s.size_bytes)
    })
}

/// Human size with decimal rounding for summary display only (the JSON
/// contract carries exact bytes): 256060514304 -> `256G`, 4294967296 -> `4G`.
fn human_size(bytes: u64) -> String {
    const TB: u64 = 1_000_000_000_000;
    const GB: u64 = 1_000_000_000;
    const MB: u64 = 1_000_000;
    const KB: u64 = 1_000;
    match bytes {
        b if b >= TB => format!("{}T", b / TB),
        b if b >= GB => format!("{}G", (b as f64 / GB as f64).round() as u64),
        b if b >= MB => format!("{}M", (b as f64 / MB as f64).round() as u64),
        b if b >= KB => format!("{}K", (b as f64 / KB as f64).round() as u64),
        b => format!("{b}B"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn fixture() -> Output {
        Output {
            command: "scan".into(),
            generated_at: "2026-02-14T10:00:00Z".into(),
            os: Os {
                distro: Some("Debian GNU/Linux 13 (trixie)".into()),
                kernel: Some("6.12.107+deb13-amd64".into()),
                arch: Some("x86_64".into()),
                hostname: Some("fixy".into()),
            },
            cpu: Cpu {
                model: Some("AMD Ryzen 3 5300U with Radeon Graphics".into()),
                vendor: Some("AMD".into()),
                cores: Some(4),
                threads: Some(8),
                max_mhz: Some(3900.0),
            },
            memory: Memory {
                total_bytes: Some(11_833_274_368),
                available_bytes: Some(5_358_608_384),
                swap_total_bytes: Some(16_455_294_976),
                zram_total_bytes: Some(4_294_963_200),
            },
            pci_devices: vec![
                PciDevice {
                    slot: "01:00.0".into(),
                    class: "Network controller".into(),
                    vendor: "Realtek Semiconductor Co., Ltd.".into(),
                    device: "RTL8821CE 802.11ac PCIe Wireless Network Adapter".into(),
                    driver_in_use: Some("rtw_8821ce".into()),
                    modules: vec!["rtw88_8821ce".into()],
                    category: "wireless".into(),
                },
                PciDevice {
                    slot: "03:00.0".into(),
                    class: "VGA compatible controller".into(),
                    vendor: "Advanced Micro Devices, Inc. [AMD/ATI]".into(),
                    device: "Lucienne".into(),
                    driver_in_use: Some("amdgpu".into()),
                    modules: vec!["amdgpu".into()],
                    category: "gpu".into(),
                },
            ],
            usb_devices: Vec::new(),
            storage: vec![
                StorageDevice {
                    name: "zram0".into(),
                    model: None,
                    size_bytes: Some(4_294_967_296),
                    device_type: "disk".into(),
                    transport: None,
                    removable: Some(false),
                },
                StorageDevice {
                    name: "nvme0n1".into(),
                    model: Some("KBG40ZNV256G KIOXIA".into()),
                    size_bytes: Some(256_060_514_304),
                    device_type: "disk".into(),
                    transport: Some("nvme".into()),
                    removable: Some(false),
                },
            ],
            network_links: Vec::new(),
            rfkill_devices: Vec::new(),
            dmi: Dmi {
                available: false,
                board_vendor: None,
                board_name: None,
                bios_version: None,
            },
            errors: Vec::new(),
        }
    }

    #[test]
    fn renders_demo_shape() {
        let out = render(&fixture());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "GPU: AMD Lucienne (iGPU) — driver: amdgpu ✓");
        assert_eq!(lines[1], "WiFi: Realtek RTL8821CE — driver: rtw_8821ce ✓");
        assert_eq!(
            lines[2],
            "Kernel: 6.12.107+deb13-amd64 · x86_64 · 4 cores / 8 threads · NVMe KIOXIA 256G + zram 4G"
        );
    }

    #[test]
    fn missing_gpu_and_driver_degrade() {
        let mut out = fixture();
        out.pci_devices.clear();
        out.pci_devices.push(PciDevice {
            slot: "00:1f.6".into(),
            class: "Ethernet controller".into(),
            vendor: "Intel Corporation".into(),
            device: "Ethernet Connection (2) I219-V".into(),
            driver_in_use: None,
            modules: vec!["e1000e".into()],
            category: "ethernet".into(),
        });
        let rendered = render(&out);
        assert!(rendered.contains("GPU: not detected"));
        assert!(rendered.contains("WiFi: not detected"));
    }

    #[test]
    fn human_sizes_round_decimal() {
        assert_eq!(human_size(256_060_514_304), "256G");
        assert_eq!(human_size(4_294_967_296), "4G");
        assert_eq!(human_size(512), "512B");
        assert_eq!(human_size(1_500_000_000), "2G");
    }

    #[test]
    fn vendor_shortening() {
        assert_eq!(
            vendor_short("Advanced Micro Devices, Inc. [AMD/ATI]"),
            "AMD"
        );
        assert_eq!(vendor_short("Realtek Semiconductor Co., Ltd."), "Realtek");
        assert_eq!(vendor_short("Intel Corporation"), "Intel");
        assert_eq!(vendor_short("Mystery Chips Inc."), "Mystery");
    }

    #[test]
    fn renders_multi_gpu_lines() {
        let mut out = fixture();
        out.pci_devices.push(PciDevice {
            slot: "01:00.0".into(),
            class: "VGA compatible controller".into(),
            vendor: "NVIDIA Corporation".into(),
            device: "GA106M [GeForce RTX 3060 Mobile]".into(),
            driver_in_use: Some("nvidia".into()),
            modules: vec!["nvidia".into()],
            category: "gpu".into(),
        });
        let rendered = render(&out);
        assert!(rendered.contains("GPU: AMD Lucienne (iGPU) — driver: amdgpu ✓"));
        assert!(rendered.contains("GPU: NVIDIA GA106M (dGPU) — driver: nvidia ✓"));
    }
}
