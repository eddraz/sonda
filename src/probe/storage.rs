//! Block device probe via `lsblk` (top-level disks).

use crate::model::{ProbeError, StorageDevice};
use crate::shell;

pub(crate) fn probe(errors: &mut Vec<ProbeError>) -> Vec<StorageDevice> {
    if !shell::which("lsblk") {
        return Vec::new();
    }
    match shell::run("LC_ALL=C lsblk -bno NAME,MODEL,SIZE,TYPE,TRAN,RM -l") {
        Ok(text) => parse_storage(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "lsblk".into(),
                message: e.to_string(),
            });
            Vec::new()
        }
    }
}

/// Transports lsblk can report; used to disambiguate an empty TRAN column.
const TRANSPORTS: [&str; 10] = [
    "nvme", "sata", "sas", "usb", "mmc", "fc", "iscsi", "fcoe", "rbd", "nbd",
];

/// Parse flat `lsblk -bno NAME,MODEL,SIZE,TYPE,TRAN,RM -l` rows, keeping only
/// whole disks. Columns are space-padded and both MODEL and TRAN may be
/// empty, so rows are consumed right-to-left: RM, TRAN (when present), TYPE,
/// SIZE, then NAME plus any MODEL words.
fn parse_storage(text: &str) -> Vec<StorageDevice> {
    text.lines()
        .filter_map(|line| {
            let mut tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() < 4 {
                return None;
            }
            let removable = tokens.pop().and_then(|t| match t {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            });
            // TRAN is optional (e.g. zram has none): peek the last token and
            // only consume it when it is a known transport, so the TYPE token
            // is never swallowed.
            let transport = tokens
                .last()
                .filter(|t| TRANSPORTS.contains(t))
                .map(|t| (*t).to_owned());
            if transport.is_some() {
                tokens.pop();
            }
            let device_type = tokens.pop()?.to_owned();
            if device_type != "disk" {
                return None;
            }
            let size_bytes = tokens.pop().and_then(|t| t.parse().ok());
            let name = (*tokens.first()?).to_owned();
            let model = tokens.split_off(1).join(" ").trim().to_owned();
            Some(StorageDevice {
                name,
                model: (!model.is_empty()).then_some(model),
                size_bytes,
                device_type,
                transport,
                removable,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LSBLK: &str = "zram0                           4294967296 disk        0\nnvme0n1   KBG40ZNV256G KIOXIA 256060514304 disk nvme   0\nnvme0n1p1                       1023410176 part nvme   0\n";

    #[test]
    fn keeps_disks_with_and_without_model_transport() {
        let devices = parse_storage(LSBLK);
        assert_eq!(devices.len(), 2);
        let zram = &devices[0];
        assert_eq!(zram.name, "zram0");
        assert_eq!(zram.model, None);
        assert_eq!(zram.transport, None);
        assert_eq!(zram.size_bytes, Some(4_294_967_296));
        let nvme = &devices[1];
        assert_eq!(nvme.name, "nvme0n1");
        assert_eq!(nvme.model.as_deref(), Some("KBG40ZNV256G KIOXIA"));
        assert_eq!(nvme.transport.as_deref(), Some("nvme"));
        assert_eq!(nvme.size_bytes, Some(256_060_514_304));
        assert_eq!(nvme.removable, Some(false));
    }

    #[test]
    fn sata_disk_without_model() {
        let devices = parse_storage("sda                      500107862016 disk sata   0\n");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "sda");
        assert_eq!(devices[0].model, None);
        assert_eq!(devices[0].transport.as_deref(), Some("sata"));
    }

    #[test]
    fn skips_partitions_and_garbage() {
        assert!(parse_storage("nvme0n1p1   1023410176 part nvme   0\nshort line\n").is_empty());
    }
}
