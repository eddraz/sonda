//! USB device probe via `lsusb`.

use crate::model::{ProbeError, UsbDevice};
use crate::shell;

pub(crate) fn probe(errors: &mut Vec<ProbeError>) -> Vec<UsbDevice> {
    if !shell::which("lsusb") {
        return Vec::new();
    }
    match shell::run("LC_ALL=C lsusb") {
        Ok(text) => parse_usb(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "lsusb".into(),
                message: e.to_string(),
            });
            Vec::new()
        }
    }
}

/// Parse lines like `Bus 001 Device 003: ID 0bda:b00e Realtek Bluetooth Radio`.
fn parse_usb(text: &str) -> Vec<UsbDevice> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == "Bus").then_some(())?;
            let bus = fields.next()?.to_owned();
            (fields.next()? == "Device").then_some(())?;
            let device = fields.next()?.trim_end_matches(':').to_owned();
            let rest = line.split(" ID ").nth(1)?;
            let (id, name) = rest.split_once(' ')?;
            let (vendor_id, product_id) = id.split_once(':')?;
            Some(UsbDevice {
                bus,
                device,
                vendor_id: vendor_id.to_owned(),
                product_id: product_id.to_owned(),
                name: name.trim().to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsusb_lines() {
        let text = "Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub\nBus 001 Device 003: ID 0bda:b00e Realtek Semiconductor Corp. Bluetooth Radio \n";
        let devices = parse_usb(text);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[1].bus, "001");
        assert_eq!(devices[1].device, "003");
        assert_eq!(devices[1].vendor_id, "0bda");
        assert_eq!(devices[1].product_id, "b00e");
        assert_eq!(
            devices[1].name,
            "Realtek Semiconductor Corp. Bluetooth Radio"
        );
    }

    #[test]
    fn skips_unparseable_lines() {
        let devices = parse_usb("garbage\nBus 001 Device 002: ID 30c9:0013\n");
        // Line without a name after the id is skipped.
        assert!(devices.is_empty());
    }
}
