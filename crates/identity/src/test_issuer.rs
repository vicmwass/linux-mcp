//! Deterministic test issuer metadata and public/private fixture material.
//! These values are intentionally invalid for production and contain no secrets.

pub const ISSUER_URL: &str = "https://issuer.fixture.invalid";
pub const AUDIENCE: &str = "linux-mcp-fixture";
pub const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\nfixture-public-key-only\n-----END PUBLIC KEY-----\n";
pub const PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nfixture-private-key-not-for-production\n-----END PRIVATE KEY-----\n";

pub fn metadata() -> (&'static str, &'static str, &'static str) {
    (ISSUER_URL, AUDIENCE, PUBLIC_KEY_PEM)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_is_deterministic_and_marked_nonproduction() {
        assert!(ISSUER_URL.ends_with(".invalid"));
        assert!(PRIVATE_KEY_PEM.contains("not-for-production"));
        assert_eq!(metadata().0, ISSUER_URL);
    }
}
