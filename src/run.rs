//! Scan orchestration: run every probe, assemble the contract v1 output.

use crate::model::Output;
use crate::probe::{self, core, dmi};
use crate::timefmt;

/// Run a full hardware scan. Never fails: missing information stays null and
/// probe failures land in `errors`.
pub fn run_scan() -> Output {
    let mut errors = Vec::new();
    let pci_devices = probe::pci::probe(&mut errors);
    let usb_devices = probe::usb::probe(&mut errors);
    let storage = probe::storage::probe(&mut errors);
    let network_links = probe::net::probe(&mut errors);
    let rfkill_devices = probe::rfkill::probe(&mut errors);
    Output {
        command: "scan".into(),
        generated_at: timefmt::now_rfc3339(),
        os: core::os_info(),
        cpu: core::cpu_info(&mut errors),
        memory: core::memory_info(),
        pci_devices,
        usb_devices,
        storage,
        network_links,
        rfkill_devices,
        dmi: dmi::probe(),
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
        // Sections exist; emptiness depends on the host's tooling.
        assert!(!out.dmi.available || out.dmi.bios_version.is_some());
    }
}
