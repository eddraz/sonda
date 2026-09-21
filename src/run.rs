//! Scan orchestration: run every probe, assemble the contract v1 output.

use crate::model::{Dmi, Output};
use crate::probe;
use crate::timefmt;

/// Run a full hardware scan. Never fails: missing information stays null and
/// probe failures land in `errors`.
pub fn run_scan() -> Output {
    let mut errors = Vec::new();
    Output {
        command: "scan".into(),
        generated_at: timefmt::now_rfc3339(),
        os: probe::core::os_info(),
        cpu: probe::core::cpu_info(&mut errors),
        memory: probe::core::memory_info(),
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
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_has_contract_shape() {
        let out = run_scan();
        assert_eq!(out.command, "scan");
        assert!(out.generated_at.ends_with('Z'));
        assert!(out.pci_devices.is_empty());
        assert!(out.usb_devices.is_empty());
        assert!(out.storage.is_empty());
        assert!(out.network_links.is_empty());
        assert!(out.rfkill_devices.is_empty());
        assert!(!out.dmi.available);
    }
}
