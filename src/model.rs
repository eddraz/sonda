//! Domain model: scan result sections and the frozen v1 JSON output contract.
//!
//! Field names and field order are part of the contract and asserted by the
//! tests below. Every piece of hardware information that cannot be probed
//! (missing binary, missing permission, missing kernel interface) serializes
//! as `null`; the scan itself never fails the process.

use serde::Serialize;

/// Operating system identity.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Os {
    /// `PRETTY_NAME` from /etc/os-release; null when unreadable.
    pub distro: Option<String>,
    /// Kernel release, e.g. `6.12.107+deb13-amd64`.
    pub kernel: Option<String>,
    /// Machine architecture, e.g. `x86_64`.
    pub arch: Option<String>,
    /// Host nodename.
    pub hostname: Option<String>,
}

/// CPU summary as reported by `lscpu`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Cpu {
    pub model: Option<String>,
    /// Human vendor name (`AuthenticAMD` -> `AMD`, `GenuineIntel` -> `Intel`);
    /// passthrough for unknown vendors.
    pub vendor: Option<String>,
    /// Physical cores (cores per socket x sockets).
    pub cores: Option<u32>,
    /// Logical threads (`CPU(s)` in lscpu).
    pub threads: Option<u32>,
    /// Maximum frequency in MHz.
    pub max_mhz: Option<f64>,
}

/// Memory summary from /proc/meminfo and /proc/swaps.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Memory {
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    /// Sum of zram swap devices; null when zram is not in use.
    pub zram_total_bytes: Option<u64>,
}

/// One PCI device with its kernel driver state (`lspci -k`).
#[derive(Debug, Clone, Serialize)]
pub struct PciDevice {
    /// PCI slot address, e.g. `03:00.0`.
    pub slot: String,
    /// pci.ids class name, e.g. `Network controller`.
    pub class: String,
    /// pci.ids vendor name, e.g. `Realtek Semiconductor Co., Ltd.`.
    pub vendor: String,
    /// pci.ids device name.
    pub device: String,
    /// Kernel driver currently bound to the device; null when none is loaded.
    /// Diagnostically meaningful, not an error.
    pub driver_in_use: Option<String>,
    /// Kernel modules that claim this device.
    pub modules: Vec<String>,
    /// Coarse bucket: gpu | wireless | ethernet | audio | storage | other.
    pub category: String,
}

/// One USB device (`lsusb`).
#[derive(Debug, Clone, Serialize)]
pub struct UsbDevice {
    /// Bus number with leading zeros preserved, e.g. `001`.
    pub bus: String,
    /// Device number on the bus, e.g. `003`.
    pub device: String,
    /// USB vendor id, e.g. `8087`.
    pub vendor_id: String,
    /// USB product id, e.g. `0026`.
    pub product_id: String,
    /// lsusb-resolved name.
    pub name: String,
}

/// One block device (`lsblk`, top-level disks only).
#[derive(Debug, Clone, Serialize)]
pub struct StorageDevice {
    pub name: String,
    /// Device model string; null when the device has none (e.g. zram).
    pub model: Option<String>,
    pub size_bytes: Option<u64>,
    /// lsblk TYPE, e.g. `disk`.
    #[serde(rename = "type")]
    pub device_type: String,
    /// lsblk TRAN, e.g. `nvme`, `sata`, `usb`; null when not reported.
    pub transport: Option<String>,
    pub removable: Option<bool>,
}

/// One network interface (`ip -br link`).
#[derive(Debug, Clone, Serialize)]
pub struct NetworkLink {
    pub name: String,
    /// Operational state, e.g. `UP`, `DOWN`, `UNKNOWN`.
    pub state: String,
}

/// One radio kill switch entry (`rfkill`).
#[derive(Debug, Clone, Serialize)]
pub struct RfkillDevice {
    /// rfkill device name, e.g. `phy0: Wireless LAN`.
    pub name: String,
    /// rfkill type, e.g. `wlan`, `bluetooth`.
    #[serde(rename = "type")]
    pub device_type: String,
    pub soft_blocked: Option<bool>,
    pub hard_blocked: Option<bool>,
}

/// DMI/board data from sysfs or `dmidecode`. Requires root; null without it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Dmi {
    /// True when dmidecode ran successfully and values are populated.
    pub available: bool,
    pub board_vendor: Option<String>,
    pub board_name: Option<String>,
    pub bios_version: Option<String>,
}

/// A probe-level failure; the scan still exits 0 with this recorded.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeError {
    /// Probe name, e.g. `lscpu`, `lspci`.
    pub probe: String,
    pub message: String,
}

/// Frozen v1 JSON output contract. Field names and order are part of the contract.
#[derive(Debug, Serialize)]
pub struct Output {
    pub command: String,
    pub generated_at: String,
    pub os: Os,
    pub cpu: Cpu,
    pub memory: Memory,
    pub pci_devices: Vec<PciDevice>,
    pub usb_devices: Vec<UsbDevice>,
    pub storage: Vec<StorageDevice>,
    pub network_links: Vec<NetworkLink>,
    pub rfkill_devices: Vec<RfkillDevice>,
    pub dmi: Dmi,
    pub errors: Vec<ProbeError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_and_memory_null_fields_serialize_as_null() {
        let os = Os::default();
        let s = serde_json::to_string(&os).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["distro"], serde_json::json!(null));
        assert_eq!(v["kernel"], serde_json::json!(null));
        assert_eq!(v["arch"], serde_json::json!(null));
        assert_eq!(v["hostname"], serde_json::json!(null));

        let mem = Memory::default();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&mem).unwrap()).unwrap();
        assert_eq!(v["zram_total_bytes"], serde_json::json!(null));
    }

    #[test]
    fn pci_device_keeps_driver_null_and_modules_list() {
        let dev = PciDevice {
            slot: "01:00.0".into(),
            class: "Network controller".into(),
            vendor: "Realtek Semiconductor Co., Ltd.".into(),
            device: "RTL8821CE 802.11ac PCIe Wireless Network Adapter".into(),
            driver_in_use: None,
            modules: vec!["rtw88_8821ce".into()],
            category: "wireless".into(),
        };
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&dev).unwrap()).unwrap();
        assert_eq!(v["driver_in_use"], serde_json::json!(null));
        assert_eq!(v["modules"], serde_json::json!(["rtw88_8821ce"]));
        assert_eq!(v["category"], serde_json::json!("wireless"));
    }

    #[test]
    fn storage_renames_kind_to_type() {
        let dev = StorageDevice {
            name: "nvme0n1".into(),
            model: Some("KBG40ZNV256G KIOXIA".into()),
            size_bytes: Some(256_060_514_304),
            device_type: "disk".into(),
            transport: Some("nvme".into()),
            removable: Some(false),
        };
        let s = serde_json::to_string(&dev).unwrap();
        assert!(s.contains("\"type\":\"disk\""));
        assert!(!s.contains("device_type"));
    }

    #[test]
    fn dmi_without_root_serializes_nulls() {
        let dmi = Dmi {
            available: false,
            board_vendor: None,
            board_name: None,
            bios_version: None,
        };
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&dmi).unwrap()).unwrap();
        assert_eq!(v["available"], serde_json::json!(false));
        assert_eq!(v["board_vendor"], serde_json::json!(null));
        assert_eq!(v["bios_version"], serde_json::json!(null));
    }

    #[test]
    fn output_field_order_matches_contract() {
        let out = Output {
            command: "scan".into(),
            generated_at: "2026-02-14T10:00:00Z".into(),
            os: Os::default(),
            cpu: Cpu::default(),
            memory: Memory::default(),
            pci_devices: Vec::new(),
            usb_devices: Vec::new(),
            storage: Vec::new(),
            network_links: Vec::new(),
            rfkill_devices: Vec::new(),
            dmi: Dmi {
                available: false,
                board_vendor: None,
                board_name: None,
                bios_version: None,
            },
            errors: Vec::new(),
        };
        // serde_json::to_value normalizes into a sorted map, so field order
        // must be asserted against the serialized string itself.
        let s = serde_json::to_string(&out).unwrap();
        let keys = [
            "\"command\":",
            "\"generated_at\":",
            "\"os\":",
            "\"cpu\":",
            "\"memory\":",
            "\"pci_devices\":",
            "\"usb_devices\":",
            "\"storage\":",
            "\"network_links\":",
            "\"rfkill_devices\":",
            "\"dmi\":",
            "\"errors\":",
        ];
        let mut last = 0;
        for k in keys {
            let pos = s
                .find(k)
                .unwrap_or_else(|| panic!("key {k} missing in {s}"));
            assert!(pos > last, "key {k} out of order in {s}");
            last = pos;
        }
    }
}
