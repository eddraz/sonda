//! Radio kill switch probe via `rfkill`. The binary is frequently absent;
//! policy is a silent skip with an empty list.

use crate::model::{ProbeError, RfkillDevice};
use crate::shell;

pub(crate) fn probe(errors: &mut Vec<ProbeError>) -> Vec<RfkillDevice> {
    if !shell::which("rfkill") {
        return Vec::new();
    }
    match shell::run("LC_ALL=C rfkill list") {
        Ok(text) => parse_rfkill(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "rfkill".into(),
                message: e.to_string(),
            });
            Vec::new()
        }
    }
}

/// Parse blocks like:
/// ```text
/// 0: phy0: Wireless LAN
///     Type: wlan\n///     Soft blocked: no\n///     Hard blocked: no
/// ```
fn parse_rfkill(text: &str) -> Vec<RfkillDevice> {
    let mut devices = Vec::new();
    for block in text.split('\n') {
        let line = block.trim();
        if line.is_empty() {
            continue;
        }
        if !block.starts_with([' ', '\t']) {
            // Header: `0: phy0: Wireless LAN`.
            let (_, rest) = line.split_once(": ").unwrap_or(("", ""));
            devices.push(RfkillDevice {
                name: rest.to_owned(),
                device_type: "unknown".to_owned(),
                soft_blocked: None,
                hard_blocked: None,
            });
            continue;
        }
        let Some(current) = devices.last_mut() else {
            continue;
        };
        if let Some(kind) = line.strip_prefix("Type: ") {
            current.device_type = kind.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("Soft blocked: ") {
            current.soft_blocked = parse_bool(value);
        } else if let Some(value) = line.strip_prefix("Hard blocked: ") {
            current.hard_blocked = parse_bool(value);
        }
    }
    devices
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfkill_blocks() {
        let text = "0: phy0: Wireless LAN\n\tType: wlan\n\tSoft blocked: no\n\tHard blocked: no\n1: hci0: Bluetooth\n\tType: bluetooth\n\tSoft blocked: yes\n\tHard blocked: no\n";
        let devices = parse_rfkill(text);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "phy0: Wireless LAN");
        assert_eq!(devices[0].device_type, "wlan");
        assert_eq!(devices[0].soft_blocked, Some(false));
        assert_eq!(devices[1].name, "hci0: Bluetooth");
        assert_eq!(devices[1].soft_blocked, Some(true));
        assert_eq!(devices[1].hard_blocked, Some(false));
    }

    #[test]
    fn unknown_bool_values_stay_none() {
        let text = "0: phy0: Wireless LAN\n\tSoft blocked: maybe\n";
        let devices = parse_rfkill(text);
        assert_eq!(devices[0].soft_blocked, None);
        assert_eq!(devices[0].device_type, "unknown");
    }
}
