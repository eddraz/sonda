//! Scan orchestration: run every probe, assemble the contract v1 output.

use crate::model::Output;
use crate::probe::{self, core, dmi};
use crate::timefmt;

/// Run a full hardware scan. Never fails: missing information stays null and
/// probe failures land in `errors`.
///
/// Probes run concurrently via `std::thread::scope` without adding any external
/// dependencies. Probe errors are appended deterministically in contract order.
pub fn run_scan() -> Output {
    let generated_at = timefmt::now_rfc3339();

    let (
        os,
        (cpu, mut cpu_errs),
        memory,
        (pci_devices, mut pci_errs),
        (usb_devices, mut usb_errs),
        (storage, mut storage_errs),
        (network_links, mut net_errs),
        (rfkill_devices, mut rfkill_errs),
        dmi,
    ) = std::thread::scope(|s| {
        let h_os = s.spawn(core::os_info);
        let h_cpu = s.spawn(|| {
            let mut errs = Vec::new();
            let res = core::cpu_info(&mut errs);
            (res, errs)
        });
        let h_mem = s.spawn(core::memory_info);
        let h_pci = s.spawn(|| {
            let mut errs = Vec::new();
            let res = probe::pci::probe(&mut errs);
            (res, errs)
        });
        let h_usb = s.spawn(|| {
            let mut errs = Vec::new();
            let res = probe::usb::probe(&mut errs);
            (res, errs)
        });
        let h_storage = s.spawn(|| {
            let mut errs = Vec::new();
            let res = probe::storage::probe(&mut errs);
            (res, errs)
        });
        let h_net = s.spawn(|| {
            let mut errs = Vec::new();
            let res = probe::net::probe(&mut errs);
            (res, errs)
        });
        let h_rfkill = s.spawn(|| {
            let mut errs = Vec::new();
            let res = probe::rfkill::probe(&mut errs);
            (res, errs)
        });
        let h_dmi = s.spawn(dmi::probe);

        (
            h_os.join().unwrap_or_default(),
            h_cpu.join().unwrap_or_default(),
            h_mem.join().unwrap_or_default(),
            h_pci.join().unwrap_or_default(),
            h_usb.join().unwrap_or_default(),
            h_storage.join().unwrap_or_default(),
            h_net.join().unwrap_or_default(),
            h_rfkill.join().unwrap_or_default(),
            h_dmi.join().unwrap_or_default(),
        )
    });

    // Error ordering is deterministic and matches the contract probe sequence.
    let mut errors = Vec::new();
    errors.append(&mut pci_errs);
    errors.append(&mut usb_errs);
    errors.append(&mut storage_errs);
    errors.append(&mut net_errs);
    errors.append(&mut rfkill_errs);
    errors.append(&mut cpu_errs);

    Output {
        command: "scan".into(),
        generated_at,
        os,
        cpu,
        memory,
        pci_devices,
        usb_devices,
        storage,
        network_links,
        rfkill_devices,
        dmi,
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
