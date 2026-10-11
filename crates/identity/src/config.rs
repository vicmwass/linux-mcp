use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    pub issuer_url: String,
    pub audience: String,
    #[serde(default = "default_revocation_seconds")]
    pub revocation_check_seconds: u64,
    #[serde(default)]
    pub allow_test_issuer: bool,
}

impl IdentityConfig {
    pub fn validate(&self, test_profile: bool) -> Result<(), IdentityConfigError> {
        if self.issuer_url.trim().is_empty() || self.audience.trim().is_empty() {
            return Err(IdentityConfigError::MissingIssuerConfiguration);
        }
        if self.revocation_check_seconds == 0 || self.revocation_check_seconds > 60 {
            return Err(IdentityConfigError::InvalidRevocationWindow);
        }
        if self.allow_test_issuer && !test_profile {
            return Err(IdentityConfigError::TestIssuerInProduction);
        }
        Ok(())
    }
}

fn default_revocation_seconds() -> u64 { 60 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityConfigError {
    #[error("issuer_url and audience are required")]
    MissingIssuerConfiguration,
    #[error("revocation_check_seconds must be between 1 and 60")]
    InvalidRevocationWindow,
    #[error("test issuer is forbidden outside an explicit test profile")]
    TestIssuerInProduction,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_rejects_test_issuer() {
        let c = IdentityConfig { issuer_url: "https://issuer.test".into(), audience: "mcp".into(), revocation_check_seconds: 60, allow_test_issuer: true };
        assert_eq!(c.validate(false), Err(IdentityConfigError::TestIssuerInProduction));
        assert!(c.validate(true).is_ok());
    }
}
