//! DMI board/BIOS probe via `dmidecode`. Requires root: without it the
//! command fails with a permission error and the section stays null.

use crate::model::Dmi;
use crate::shell;

pub(crate) fn probe() -> Dmi {
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
}
