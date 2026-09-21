//! Network interface probe via `ip -br link`.

use crate::model::{NetworkLink, ProbeError};
use crate::shell;

pub(crate) fn probe(errors: &mut Vec<ProbeError>) -> Vec<NetworkLink> {
    if !shell::which("ip") {
        return Vec::new();
    }
    match shell::run("LC_ALL=C ip -br link") {
        Ok(text) => parse_links(&text),
        Err(e) => {
            errors.push(ProbeError {
                probe: "ip".into(),
                message: e.to_string(),
            });
            Vec::new()
        }
    }
}

/// Parse brief rows: `wlo1 UP 86:f2:f5:9f:21:f1 <BROADCAST,MULTICAST,UP>`.
fn parse_links(text: &str) -> Vec<NetworkLink> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.to_owned();
            let state = fields.next()?.to_owned();
            Some(NetworkLink { name, state })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brief_link_rows() {
        let text = "lo               UNKNOWN        00:00:00:00:00:00 <LOOPBACK,UP,LOWER_UP> \nwlo1             UP             86:f2:f5:9f:21:f1 <BROADCAST,MULTICAST,UP,LOWER_UP> \n";
        let links = parse_links(text);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].name, "lo");
        assert_eq!(links[0].state, "UNKNOWN");
        assert_eq!(links[1].name, "wlo1");
        assert_eq!(links[1].state, "UP");
    }

    #[test]
    fn skips_empty_and_short_lines() {
        assert!(parse_links("\nonlyname\n").is_empty());
    }
}
