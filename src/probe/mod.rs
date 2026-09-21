//! Hardware probes. Each probe reads /proc and /etc files directly through
//! std::fs and shells out to system tools exclusively via `crate::shell`.
//!
//! Policy: a probe whose backing binary is missing is skipped silently (its
//! fields stay null); a probe whose binary exists but fails records a
//! `ProbeError` into the scan's `errors` list. No probe ever panics or aborts
//! the scan.

pub(crate) mod core;

/// Read a file and return its trimmed contents, or None when unreadable.
pub(crate) fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
}
