//! PCI device probe: `lspci` with kernel driver state.
//!
//! Two sources, both pinned to the C locale:
//! - `lspci -k -m` (machine-readable, quoted fields) gives slot, class,
//!   vendor and device names without guessing name boundaries.
//! - `lspci -k` (human text) is the only source of `Kernel driver in use`
//!   and `Kernel modules`; parsed per slot and merged back.

use std::collections::BTreeMap;

use crate::model::{PciDevice, ProbeError};
use crate::shell;

pub(crate) fn probe(errors: &mut Vec<ProbeError>) -> Vec<PciDevice> {
    if !shell::which("lspci") {
        return Vec::new();
    }
    let machine = match shell::run("LC_ALL=C lspci -k -m") {
        Ok(text) => parse_machine(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "lspci".into(),
                message: e.to_string(),
            });
            return Vec::new();
        }
    };
    let details = shell::run("LC_ALL=C lspci -k")
        .map(|text| parse_details(&text))
        .unwrap_or_default();

    machine
        .into_iter()
        .map(|(slot, class, vendor, device)| {
            let (driver_in_use, modules) = details
                .get(&slot)
                .map(|d| (d.0.clone(), d.1.clone()))
                .unwrap_or((None, Vec::new()));
            PciDevice {
                slot,
                driver_in_use,
                modules,
                category: category(&class).to_owned(),
                class,
                vendor,
                device,
            }
        })
        .collect()
}

/// Parse machine-readable lines: `03:00.0 "Class" "Vendor" "Device" ...`.
fn parse_machine(text: &str) -> Vec<(String, String, String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let slot = parts.next()?.to_owned();
            let mut quoted = QuotedTokens::new(parts.next()?);
            let class = quoted.next()?;
            let vendor = quoted.next()?;
            let device = quoted.next()?;
            Some((slot, class, vendor, device))
        })
        .collect()
}

/// Iterator over double-quoted fields of one lspci machine line.
struct QuotedTokens<'a> {
    rest: &'a str,
}

impl<'a> QuotedTokens<'a> {
    fn new(rest: &'a str) -> Self {
        Self { rest }
    }

    fn next(&mut self) -> Option<String> {
        let start = self.rest.find('"')? + 1;
        let end = self.rest[start..].find('"')? + start;
        let value = self.rest[start..end].to_owned();
        self.rest = &self.rest[end + 1..];
        Some(value)
    }
}

type Details = BTreeMap<String, (Option<String>, Vec<String>)>;

/// Parse human `lspci -k` blocks for driver and module names per slot.
fn parse_details(text: &str) -> Details {
    let mut details = Details::new();
    let mut slot: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            // Device header: `03:00.0 Network controller: Vendor Device`.
            let address = trimmed.split_whitespace().next().unwrap_or("");
            if address.contains('.') && address.contains(':') {
                slot = Some(address.to_owned());
            }
            continue;
        }
        let Some(current) = slot.as_deref() else {
            continue;
        };
        let entry = details.entry(current.to_owned()).or_default();
        if let Some(driver) = trimmed.strip_prefix("Kernel driver in use: ") {
            entry.0 = Some(driver.trim().to_owned());
        } else if let Some(modules) = trimmed.strip_prefix("Kernel modules: ") {
            entry.1 = modules
                .split(',')
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }
    details
}

/// Map a pci.ids class name to a coarse diagnostic category.
fn category(class: &str) -> &'static str {
    let c = class.to_ascii_lowercase();
    if c.contains("vga") || c.contains("display") || c.contains("3d controller") {
        "gpu"
    } else if c.contains("network controller") {
        "wireless"
    } else if c.contains("ethernet") {
        "ethernet"
    } else if c.contains("audio") {
        "audio"
    } else if c.contains("non-volatile memory")
        || c.contains("sata")
        || c.contains("ide")
        || c.contains("raid")
    {
        "storage"
    } else {
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MACHINE: &str = "01:00.0 \"Network controller\" \"Realtek Semiconductor Co., Ltd.\" \"RTL8821CE 802.11ac PCIe Wireless Network Adapter\" -p00 \"Hewlett-Packard Company\" \"Device 884d\"\n03:00.0 \"VGA compatible controller\" \"Advanced Micro Devices, Inc. [AMD/ATI]\" \"Lucienne\" -rc3 -p00 \"Hewlett-Packard Company\" \"Device 887a\"\n";

    const DETAILS: &str = "01:00.0 Network controller: Realtek Semiconductor Co., Ltd. RTL8821CE 802.11ac PCIe Wireless Network Adapter\n\tSubsystem: Hewlett-Packard Company Device 884d\n\tKernel driver in use: rtw_8821ce\n\tKernel modules: rtw88_8821ce\n02:00.0 Non-Volatile memory controller: KIOXIA Corporation NVMe SSD Controller BG4 (DRAM-less)\n\tKernel driver in use: nvme\n";

    #[test]
    fn parses_quoted_machine_fields() {
        let devices = parse_machine(MACHINE);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].0, "01:00.0");
        assert_eq!(devices[0].1, "Network controller");
        assert_eq!(devices[0].2, "Realtek Semiconductor Co., Ltd.");
        assert_eq!(
            devices[0].3,
            "RTL8821CE 802.11ac PCIe Wireless Network Adapter"
        );
        assert_eq!(devices[1].3, "Lucienne");
    }

    #[test]
    fn parses_driver_and_modules_per_slot() {
        let details = parse_details(DETAILS);
        let wifi = &details["01:00.0"];
        assert_eq!(wifi.0.as_deref(), Some("rtw_8821ce"));
        assert_eq!(wifi.1, vec!["rtw88_8821ce"]);
        let nvme = &details["02:00.0"];
        assert_eq!(nvme.0.as_deref(), Some("nvme"));
        assert!(nvme.1.is_empty());
    }

    #[test]
    fn merges_driver_into_devices() {
        // probe() shells out; exercise the merge logic instead.
        let machine = parse_machine(MACHINE);
        let details = parse_details(DETAILS);
        let merged: Vec<PciDevice> = machine
            .into_iter()
            .map(|(slot, class, vendor, device)| {
                let (driver_in_use, modules) = details
                    .get(&slot)
                    .map(|d| (d.0.clone(), d.1.clone()))
                    .unwrap_or((None, Vec::new()));
                PciDevice {
                    category: category(&class).to_owned(),
                    slot,
                    class,
                    vendor,
                    device,
                    driver_in_use,
                    modules,
                }
            })
            .collect();
        assert_eq!(merged[0].driver_in_use.as_deref(), Some("rtw_8821ce"));
        assert_eq!(merged[0].category, "wireless");
        assert_eq!(merged[1].category, "gpu");
        assert_eq!(merged[1].driver_in_use, None);
    }

    #[test]
    fn class_category_mapping() {
        assert_eq!(category("VGA compatible controller"), "gpu");
        assert_eq!(category("3D controller"), "gpu");
        assert_eq!(category("Display controller"), "gpu");
        assert_eq!(category("Network controller"), "wireless");
        assert_eq!(category("Ethernet controller"), "ethernet");
        assert_eq!(category("Audio device"), "audio");
        assert_eq!(category("Non-Volatile memory controller"), "storage");
        assert_eq!(category("SATA controller"), "storage");
        assert_eq!(category("PCI bridge"), "other");
        assert_eq!(category("Host bridge"), "other");
    }
}
