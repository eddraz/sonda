//! DMI board/BIOS probe: reads /sys/class/dmi/id directly (unprivileged),
//! falling back to `dmidecode` (root-only).

use std::path::Path;

use crate::model::Dmi;
use crate::probe::read_trim;
use crate::shell;

pub(crate) fn probe() -> Dmi {
    probe_from_paths(Path::new("/sys/class/dmi/id"), probe_dmidecode)
}

pub(crate) fn probe_from_paths<F>(sysfs_dir: &Path, dmidecode_fallback: F) -> Dmi
where
    F: FnOnce() -> Dmi,
{
    if let Some(dmi) = probe_sysfs(sysfs_dir) {
        return dmi;
    }
    dmidecode_fallback()
}

fn probe_sysfs(dir: &Path) -> Option<Dmi> {
    let read_entry = |name: &str| {
        let path = dir.join(name);
        read_trim(path.to_str()?).filter(|s| !s.is_empty())
    };

    let board_vendor = read_entry("board_vendor").or_else(|| read_entry("sys_vendor"));
    let board_name = read_entry("board_name").or_else(|| read_entry("product_name"));
    let bios_version = read_entry("bios_version");

    if board_vendor.is_some() || board_name.is_some() || bios_version.is_some() {
        Some(Dmi {
            available: true,
            board_vendor,
            board_name,
            bios_version,
        })
    } else {
        None
    }
}

fn probe_dmidecode() -> Dmi {
    if !shell::which("dmidecode") {
        return unavailable();
    }
    // A single scalar query decides whether dmidecode can run at all:
    // without root it fails before printing anything.
    if shell::run("LC_ALL=C dmidecode -s bios-version").is_err() {
        return unavailable();
    }
    Dmi {
        available: true,
        board_vendor: scalar("baseboard-manufacturer"),
        board_name: scalar("baseboard-product-name"),
        bios_version: scalar("bios-version"),
    }
}

fn unavailable() -> Dmi {
    Dmi {
        available: false,
        board_vendor: None,
        board_name: None,
        bios_version: None,
    }
}

/// Run one `dmidecode -s <keyword>`; empty or failed output is null.
fn scalar(keyword: &str) -> Option<String> {
    let value = shell::run(&format!("LC_ALL=C dmidecode -s {keyword}")).ok()?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_section_has_all_nulls() {
        let dmi = unavailable();
        assert!(!dmi.available);
        assert_eq!(dmi.board_vendor, None);
        assert_eq!(dmi.board_name, None);
        assert_eq!(dmi.bios_version, None);
    }

    #[test]
    fn probe_sysfs_missing_dir_falls_back() {
        let missing = Path::new("/nonexistent/dmi/path/12345");
        let dmi = probe_from_paths(missing, unavailable);
        assert!(!dmi.available);
    }

    #[test]
    fn probe_sysfs_reads_present_entries() {
        let temp = std::env::temp_dir().join(format!("sonda_dmi_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);
        std::fs::write(temp.join("board_vendor"), "TestVendor\n").unwrap();
        std::fs::write(temp.join("board_name"), "TestBoard\n").unwrap();
        std::fs::write(temp.join("bios_version"), "v1.2.3\n").unwrap();

        let dmi = probe_from_paths(&temp, unavailable);
        assert!(dmi.available);
        assert_eq!(dmi.board_vendor.as_deref(), Some("TestVendor"));
        assert_eq!(dmi.board_name.as_deref(), Some("TestBoard"));
        assert_eq!(dmi.bios_version.as_deref(), Some("v1.2.3"));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn probe_sysfs_falls_back_to_sys_vendor_and_product_name() {
        let temp =
            std::env::temp_dir().join(format!("sonda_dmi_test_fallback_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);
        std::fs::write(temp.join("sys_vendor"), "SystemVendor\n").unwrap();
        std::fs::write(temp.join("product_name"), "LaptopModel\n").unwrap();

        let dmi = probe_from_paths(&temp, unavailable);
        assert!(dmi.available);
        assert_eq!(dmi.board_vendor.as_deref(), Some("SystemVendor"));
        assert_eq!(dmi.board_name.as_deref(), Some("LaptopModel"));
        assert_eq!(dmi.bios_version, None);

        let _ = std::fs::remove_dir_all(&temp);
    }
}
