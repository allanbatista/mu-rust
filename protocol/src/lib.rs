//! Core protocol types shared between the client and server crates.

pub mod de;
pub mod header;
pub mod packets;
pub mod ser;

/// Returns the protocol crate version string.
pub fn protocol_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_pkg() {
        assert_eq!(protocol_version(), env!("CARGO_PKG_VERSION"));
    }
}
